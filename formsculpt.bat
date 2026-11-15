@echo off
setlocal
cd /d "%~dp0"
set CMD=%1
if "%CMD%"=="" set CMD=boot
if /i "%CMD%"=="boot" goto boot
if /i "%CMD%"=="dev" goto boot
if /i "%CMD%"=="install" goto install
if /i "%CMD%"=="backend" goto backend
if /i "%CMD%"=="frontend" goto frontend
if /i "%CMD%"=="build" goto build
if /i "%CMD%"=="serve" goto serve
if /i "%CMD%"=="engine" goto engine
if /i "%CMD%"=="help" goto usage
echo [formsculpt] unknown command: %CMD%
goto usage
:boot
start "formsculpt backend" cmd /c "cd backend && boot_backend.bat"
cd frontend && call boot_frontend.bat
goto end
:install
call backend\oc_install.bat || goto end
where cargo >nul 2>nul || ( echo [formsculpt] Rust is not installed; the engine needs it. Install from https://rustup.rs & goto installfrontend )
call "%~f0" engine || goto end
:installfrontend
call frontend\oc_install.bat
goto end
:backend
cd backend && call boot_backend.bat
goto end
:frontend
cd frontend && call boot_frontend.bat
goto end
:build
cd frontend && call npm run build
goto end
:serve
cd frontend && call npm run build && cd ..\backend && call venv\Scripts\activate.bat && set "FSC_DIST=%~dp0frontend\dist" && uvicorn app.main:app --host 127.0.0.1 --port 8260
goto end
:engine
pushd native
cargo build --workspace --release --exclude fsc-py || ( popd & goto end )
popd
if not exist backend\venv\Scripts\activate.bat ( echo [formsculpt] backend venv missing; the python engine module was not built, run formsculpt install & goto end )
call backend\venv\Scripts\activate.bat
pushd native\crates\fsc-py
set "CARGO_TARGET_DIR=%~dp0native\target"
maturin develop --release
popd
goto end
:usage
echo usage: formsculpt [boot^|install^|backend^|frontend^|build^|serve^|engine^|help]
:end
endlocal
