import argparse
import re

import wikipedia

# Parse arguments.
parser = argparse.ArgumentParser(description="Wikipedia corpus generator.")
parser.add_argument(
    "-a",
    "--articles",
    type=str,
    nargs="+",
    default=[
        "Book",
        "F. Scott Fitzgerald",
        "Whisky",
        "United States",
        "Bison",
        "Banana",
        "Aurora",
        "Table tennis",
        "Hardness",
        "Fashion",
        "Language",
        "Sponges",
        "Silent film",
        "Cowboy",
        "Computer keyboard",
        "Knitting needle",
        "Genre",
        "Rivalry",
        "Pollution",
        "Frost",
        "Playground slide",
        "Argument",
    ],
    help="Wikipedia article to add to the corpus",
)
args = parser.parse_args()

heading = re.compile(r"^([=]{2,}) (.*) (\1)\n$")
typable = re.compile(
    r"[^A-Za-z0-9\`\~\-\_\=\+\[\{\]\}\\\|\;\:"
    r"\'\"\,\<\.\>\/\?\!\@\#\$\%\^\&\*\(\) \n]"
)
with open("wikipedia.txt", "w") as f:
    for article in args.articles[-1:]:  # TODO: do all articles
        page = wikipedia.page(article)
        for line in page.content.splitlines(keepends=True):
            match = heading.match(line)
            # Skip headings.
            if match:
                line = match.group(2) + "\n"
                # Stop writing lines after "See also" encountered; the rest is
                # usually of poorer quality.
                lower_line = line.lower()
                if lower_line.startswith("see also") or lower_line.startswith(
                    "gallery"
                ):
                    break
            # Remove untypable characters.
            line = typable.sub("", line)
            f.write(line)
