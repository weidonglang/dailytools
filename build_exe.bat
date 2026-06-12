@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"

if not exist .venv (
    py -3.11 -m venv .venv
)

call .venv\Scripts\activate.bat
python -m pip install --upgrade pip
python -m pip install -r requirements.txt

python -m PyInstaller --noconfirm --clean --onefile --windowed --name DevEnvManager main.py

echo.
echo Build finished. Check dist\DevEnvManager.exe
pause
