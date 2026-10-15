"""Frozen data in the checkout or installed wheel; SRL_DATA_DIR overrides both."""
import os
from pathlib import Path


def data_path(name):
    """Path to name relative to the chosen data directory."""
    if root := os.environ.get("SRL_DATA_DIR"):
        return Path(root).expanduser().resolve() / name
    package = Path(__file__).resolve().parent
    root = package / "data" if (package / "data").is_dir() else package.parent / "data"
    return root / name
