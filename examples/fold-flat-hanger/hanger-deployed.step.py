# /// script
# requires-python = ">=3.11"
# dependencies = ["build123d==0.11.0"]
# ///

"""Deployed pose entry point for the Burr fold-flat hanger."""

from pathlib import Path

from build123d import export_step

from hanger import make_hanger


def gen_step():
    return make_hanger(folded=False)


if __name__ == "__main__":
    export_step(gen_step(), Path(__file__).with_suffix(""), timestamp="2026-01-01T00:00:00")
