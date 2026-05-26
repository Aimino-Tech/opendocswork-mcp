"""Entry point for `python -m opendocswork_mcp`."""
import subprocess
import sys

def main():
    # Run the Rust binary
    subprocess.run(["opendocswork-mcp"] + sys.argv[1:])

if __name__ == "__main__":
    main()
