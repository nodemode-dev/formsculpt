@echo off
setlocal EnableExtensions
cd /d "%~dp0"
set "OPEN=0"
for %%A in (%*) do (
  if /i "%%~A"=="--open" ( set "OPEN=1" ) else if /i "%%~A"=="--help" ( goto :usage ) else if /i "%%~A"=="-h" ( goto :usage ) else ( call :usage & exit /b 2 )
)

echo.
echo [formsculpt] == checking the tools
where python >nul 2>nul || ( echo [formsculpt] Python 3.12 or newer is needed: https://www.python.org/downloads/ & exit /b 1 )
where node >nul 2>nul || ( echo [formsculpt] Node 22 or newer is needed: https://nodejs.org & exit /b 1 )
where cargo >nul 2>nul || ( echo [formsculpt] Rust is needed: https://rustup.rs & exit /b 1 )
for /f "tokens=1 delims=." %%V in ('node -p "process.versions.node"') do if %%V LSS 22 ( echo [formsculpt] Node 22 or newer is needed, this is Node %%V: https://nodejs.org & exit /b 1 )
where claude >nul 2>nul && ( echo [formsculpt] Claude Code found; the app's terminal dock and Chat can start it ) || ( echo [formsculpt] Claude Code is not installed; the app builds without it: https://claude.com/claude-code )
for /f "tokens=2" %%T in ('rustc -vV ^| findstr /b "host:"') do set "TRIPLE=%%T"

echo.
echo [formsculpt] == installing the service and the window
call backend\oc_install.bat || goto :failed
pushd frontend
call npm install --no-audit --no-fund || ( popd & goto :failed )
popd

echo.
echo [formsculpt] == installing the desktop packages
call backend\venv\Scripts\activate.bat || goto :failed
pip install -r backend\requirements_desktop.txt || goto :failed

echo.
echo [formsculpt] == building the Rust engine and its Python module
pushd native
cargo build --workspace --release --exclude fsc-py || ( popd & goto :failed )
popd
pushd native\crates\fsc-py
set "CARGO_TARGET_DIR=%~dp0native\target"
maturin develop --release || ( popd & goto :failed )
set "CARGO_TARGET_DIR="
popd

echo.
echo [formsculpt] == building the window
pushd frontend
call npm run build || ( popd & goto :failed )
popd

echo.
echo [formsculpt] == freezing the service and engine into the app's sidecar
pushd backend
if exist build\pyinstaller rmdir /s /q build\pyinstaller
if exist dist-engine rmdir /s /q dist-engine
pyinstaller --noconfirm --clean --onefile --name formsculpt-engine --distpath dist-engine --workpath build\pyinstaller --specpath build\pyinstaller --add-data "%~dp0frontend\dist;frontend_dist" --collect-submodules app --collect-submodules uvicorn --collect-all fsc_engine --hidden-import app.services.cmd_terminal --hidden-import app.services.claude_code --hidden-import app.services.cmd_chat --hidden-import winpty "%~dp0backend\formsculpt_engine.py" || ( popd & goto :failed )
popd
if not exist src-tauri\binaries mkdir src-tauri\binaries
copy /y backend\dist-engine\formsculpt-engine.exe "src-tauri\binaries\formsculpt-engine-%TRIPLE%.exe" >nul || goto :failed
"src-tauri\binaries\formsculpt-engine-%TRIPLE%.exe" --selfcheck || goto :failed

echo.
echo [formsculpt] == building the desktop app (the first build compiles several hundred Rust crates)
call frontend\node_modules\.bin\tauri build || goto :failed

echo.
echo [formsculpt] done. The installers are in src-tauri\target\release\bundle (msi and nsis)
if "%OPEN%"=="1" start "" "src-tauri\target\release\bundle"
exit /b 0

:failed
echo.
echo [formsculpt] the build stopped at the step above; fix what it says and run build_app.bat again
exit /b 1

:usage
echo build_app.bat           build the formsculpt desktop app on Windows, start to finish
echo   --open                open the folder with the installers when the build is done
exit /b 0
