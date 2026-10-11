@echo off
cd /d "%~dp0"
if not exist venv\Scripts\activate.bat (echo [formsculptor] backend venv missing, run formsculptor install & exit /b 1)
call venv\Scripts\activate.bat
uvicorn app.main:app --reload --host 127.0.0.1 --port 8196
