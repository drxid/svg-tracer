@echo off
setlocal
set "HERE=%~dp0"
if not exist "%HERE%.venv\Scripts\python.exe" (
    python -m venv "%HERE%.venv" || exit /b 1
    "%HERE%.venv\Scripts\python" -m pip install -q -r "%HERE%requirements.txt" || exit /b 1
)
"%HERE%.venv\Scripts\python" "%HERE%trace.py" %*
