# pylint: disable=missing-function-docstring
# pylint: disable=missing-module-docstring
# pylint: disable=missing-class-docstring
# pylint: disable=unsubscriptable-object
# pylint: disable=too-many-return-statements

import argparse
import itertools
import logging
from collections import deque
from dataclasses import dataclass
from typing import Iterator, Optional, Tuple, TypeVar


def parse_args() -> argparse.Namespace:
    # Parse arguments.
    parser = argparse.ArgumentParser(description="COCA corpus generator.")
    parser.add_argument(
        "--input-file",
        type=str,
        default="/Users/rabraham5/Downloads/coca-samples-wlp/wlp_mag.txt",
        help="COCA file to parse",
    )
    parser.add_argument(
        "--first-line",
        type=int,
        default=0,
        help="first line to read",
    )
    parser.add_argument(
        "--last-line",
        type=int,
        default=0,
        help="last line to read",
    )
    parser.add_argument(
        "--debug",
        action=argparse.BooleanOptionalAction,
        type=bool,
        default=True,
        help="Enable DEBUG level logging",
    )
    parser.add_argument(
        "--output-file",
        type=str,
        default="coca.txt",
        help="Output file",
    )
    return parser.parse_args()


@dataclass
class Token:
    excerpt: int
    word: str
    lemma: str
    pos: str  # "part of speech" code


def tokenize(lines: Iterator[Tuple[int, str]]) -> Iterator[Tuple[int, Token]]:
    for line_number, line in lines:
        parts = line.split()
        logging.debug("Read parts=%s", parts)

        assert len(parts) >= 2

        excerpt = int(parts[0])
        word = parts[1]

        # Skip excerpt division markers (i.e. "@@<excerpt-num>").
        if len(parts) == 2:
            logging.debug("excerpt=%s word=%s", excerpt, word)
            assert word.startswith("@@")
            assert int(word[2:]) == excerpt
            continue

        lemma = parts[2]

        # 3 part lines? Strange.
        if len(parts) == 3:
            logging.debug("excerpt=%s word=%s lemma=%s", excerpt, word, lemma)
            # Skip what I assume are omitted words.
            if word == "@" and lemma == "ii":
                continue
            if word == "&nbsp;" and lemma == "zz":
                continue

            # pylint: disable=too-many-boolean-expressions

            # Strange hyphenation.
            if (
                (word == "-a" and lemma == "at1")
                or (word == "-the" and lemma == "at")
                or (word == "-The" and lemma == "at")
                or (word == "-that" and lemma == "dd1_cst")
                or (word == "-but" and lemma == "ccb")
                or (word == "-and" and lemma == "cc")
            ):
                token = Token(excerpt, "- " + word[1:], word, lemma)
                yield line_number, token
                continue

            # Miscellaneous weirdness.
            if (
                (
                    lemma == "fo"
                    and word in ("@owjc.org", "1,150-bottle-strong")
                )
                or (lemma == "mcmc")  # phone numbers?
                or (
                    lemma == "fu"
                    and word in ("SHATZ/DREAMWORKS", "myrecipes.com/")
                )
                or (lemma == "vvg" and word == "Spatchcocking")
                or (lemma == "np1" and word == "WALL/PARAMOUNT")
                or (
                    lemma == "zz_nn" and word in ("proofdc.com", "newseum.org")
                )
                or (
                    lemma == "nnu" and word in ("www.clifton.co.uk", "grom.it")
                )
                or (lemma == "nnu2" and word == "euro-cents")
                or (lemma == "zz" and word == "--")
            ):
                token = Token(excerpt, word, word, lemma)
                yield line_number, token
                continue

            # Clean up rest of 3 part lines...
            parts.append(lemma)  # actually the "pos"
            lemma = parts[2] = ""

        # Parse actual tokens.
        assert len(parts) == 4

        pos = parts[3]

        token = Token(excerpt, word, lemma, pos)
        logging.debug("Parsed token=%s", token)
        yield line_number, token


Type = TypeVar("Type")


def window(
    generator: Iterator[Type], window_size: int
) -> Iterator[Tuple[Optional[Type], ...]]:
    result: deque[Optional[Type]] = deque([None] * window_size)
    for val in generator:
        result.popleft()
        result.append(val)
        yield tuple(result)


def get_delimiter(cur_token, prev_token: Optional[Token]) -> str:
    # No space before first word.
    assert cur_token
    if not prev_token:
        return ""

    # No space after paragraphs.
    if prev_token.word == "<p>":
        assert prev_token.lemma == "<p>"
        assert prev_token.pos == "y"
        return ""  # <p>

    # Handle () properly.
    if (
        prev_token.pos == "y"
        and prev_token.word == "("
        and prev_token.lemma == "("
    ) or (
        cur_token.pos == "y"
        and cur_token.word == ")"
        and cur_token.lemma == ")"
    ):
        return ""

    # No space before ",", ":", ";", "?", "!" or "."
    if (
        cur_token.word in (",", ":", ";", "?", "!", ".")
        and cur_token.pos == "y"
    ):
        return ""

    # Starts with "'".
    if cur_token.word.startswith("'"):
        # Contractions of "be".
        if cur_token.lemma == "be":
            if cur_token.word.startswith("'"):
                logging.debug("pos=%s", cur_token.pos)
                assert cur_token.pos in (
                    "vbm",  # I'm
                    "vbr",  # you're
                    "vbx",  # there's
                    "vbz",  # what's, that's, etc.
                    "vbz_ge",  # leaguer's
                    "vbz_ge_mc222%",  # 1990's
                    "vbz_ge_vhz@",  # House's
                    "vbz_mc222%_ge",  # 41's
                    "vbz_vhz@",  # that's
                    "vbz_vhz@_ge",  # Brigitte's
                    "vbz_zz222",  # c's
                    "vhz",  # who's
                    "vhz@",  # one's
                    "vm22",  # let's
                )
                return ""

        # Contractions of "have".
        if cur_token.lemma == "have":
            logging.debug("pos=%s", cur_token.pos)
            assert cur_token.pos in (
                "vh0",  # I've
                "vhd",  # I'd
                "vhd_vm",  # they'd
                "vhi",  # could've
                "vm",  # I'd
                "vm_vhd",  # we'd
            )
            return ""

        # Contractions of "will".
        if cur_token.lemma == "will":
            logging.debug("pos=%s", cur_token.pos)
            assert cur_token.pos in ("vm",)  # I'll
            return ""

        # Them => 'em.
        if (
            cur_token.word == "'"
            and cur_token.lemma == "'"
            and cur_token.pos in ('"@', 'ge_"@')
        ):
            return ""

        # Possessives and special plurals.
        if cur_token.word == "'s":
            assert cur_token.lemma == "'s"
            assert cur_token.pos in (
                "ge",  # Joe DiMaggio's
                "ge_vbz",  # 2003's
                "ge_vbz_vhz@",  # Poor's
                "ge_vhz@",  # archipelago's
                "mc222%",  # 1800's
                "mc222%_ge",  # 30's
                "mc222%_vbz",  # Philip II's
                "mc222%_vbz_ge",  # 463's
                "zz222",  # A's
                "zz222_vbz",  # W's
            )
            return ""

        if cur_token.word == "'S":
            assert cur_token.lemma == "'s"
            assert cur_token.pos in ("ge", "ge_vbz")
            return ""  # NASA's

        # Bare '; unfortunately there's not enough info to space this properly.
        if (
            cur_token.word == "'"
            and cur_token.lemma == "'"
            and cur_token.pos == "ge"
        ):
            return ""

        # I'm not sure what this is.
        if (
            cur_token.word == "'"
            and cur_token.lemma == "'"
            and cur_token.pos in ('"@_ge', 'ge_"')
        ):
            return " "

    assert not cur_token.word.startswith("'")

    # Starts with '"'.
    if cur_token.word.startswith('"'):
        # Bare "; unfortunately there's not enough info to space this properly.
        if (
            cur_token.word == '"'
            and cur_token.lemma == '"'
            and cur_token.pos == "y"
        ):
            return ""

    assert not cur_token.word.startswith('"')

    # Negative contractions.
    if cur_token.word == "n't":
        assert cur_token.lemma == "n't"
        assert cur_token.pos == "xx"
        return ""  # don't

    # Paragraphs.
    if cur_token.word == "<p>":
        assert cur_token.lemma == "<p>"
        assert cur_token.pos == "y"
        return ""  # <p>

    # ...
    if cur_token.word == "...":
        assert cur_token.lemma == "..."
        assert cur_token.pos == "..."
        return ""  # <p>

    return " "


def main() -> None:
    args = parse_args()

    logging.basicConfig(
        format="%(asctime)s %(levelname)s %(message)s",
        level=(logging.DEBUG if args.debug else logging.INFO),
    )

    with open(args.input_file, "r", encoding="ISO-8859-1") as input_file, open(
        args.output_file, "w"
    ) as output_file:

        lines = enumerate(
            itertools.islice(input_file, args.first_line, args.last_line),
            start=args.first_line,
        )
        try:
            for (prev, cur) in window(tokenize(lines), 2):
                assert cur
                line_number = cur[0]
                cur_token = cur[1]
                prev_token = prev[1] if prev else None

                logging.debug(
                    "Processing line_number=%d, cur_token=%s, prev_token=%s",
                    line_number,
                    cur_token,
                    prev_token,
                )

                assert cur_token
                delim = get_delimiter(cur_token, prev_token)

                # Paragraphs.
                if cur_token.word == "<p>":
                    assert cur_token.lemma == "<p>"
                    assert cur_token.pos == "y"
                    output_file.write("\n")
                    continue

                # Everything else.
                output_file.write(delim + cur_token.word)
        except AssertionError:
            logging.exception(
                "Failed on line_number=%d, cur_token=%s, prev_token=%s",
                line_number,
                cur_token,
                prev_token,
            )


if __name__ == "__main__":
    main()
