# FibRace Performance Report Notebook Setup

## Dataset

The dataset is available with Git LFS:

```bash
git lfs pull
```

Use Python 3.10+ for the virtual environment. The commands below assume Linux or
macOS shells; adapt `source` to `Scripts\\activate` if you are on Windows
PowerShell.

```bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

Once the dependencies finish installing, launch Jupyter Lab from the repository
root:

```bash
jupyter lab performance-report.ipynb
```
