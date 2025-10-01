# FibRace Performance Report Notebook Setup

## Dataset

The dataset is available with Git LFS:

```bash
git lfs pull
```

## Setup

Install [uv](https://docs.astral.sh/uv/) if you haven't already:

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
```

Once `uv` is installed, launch Jupyter Lab from the `docs/fibrace` directory.
`uv` will automatically manage the Python version and dependencies:

```bash
uv run jupyter lab performance-report.ipynb
```
