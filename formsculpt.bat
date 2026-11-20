@echo off
setlocal
set "CALLER_DIR=%cd%"
cd /d "%~dp0"
set CMD=%1
set "SUB=%~2"
if "%CMD%"=="" set CMD=boot
if /i "%CMD%"=="boot" goto boot
if /i "%CMD%"=="dev" goto boot
if /i "%CMD%"=="install" goto install
if /i "%CMD%"=="backend" goto backend
if /i "%CMD%"=="frontend" goto frontend
if /i "%CMD%"=="build" goto build
if /i "%CMD%"=="serve" goto serve
if /i "%CMD%"=="engine" goto engine
if /i "%CMD%"=="install-desktop" goto installdesktop
if /i "%CMD%"=="freeze" goto freeze
if /i "%CMD%"=="desktop" goto desktop
if /i "%CMD%"=="app" goto app
if /i "%CMD%"=="doctor" goto doctor
if /i "%CMD%"=="test" goto test
if /i "%CMD%"=="bench" goto bench
if /i "%CMD%"=="mcp" goto mcp
if /i "%CMD%"=="script" goto script
if /i "%CMD%"=="i18n" goto i18n
if /i "%CMD%"=="agents" goto agents
if /i "%CMD%"=="clean" goto clean
if /i "%CMD%"=="help" goto usage
if /i "%CMD%"=="-h" goto usage
if /i "%CMD%"=="--help" goto usage
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
if not exist frontend\node_modules ( pushd frontend & call npm install & popd )
if not exist frontend\public\engine\fsc_engine.wasm ( where cargo >nul 2>nul && ( call "%~f0" engine wasm || echo [formsculpt] the browser engine was not built; the window will need the engine service ) )
cd frontend && call npm run build
goto end
:serve
if not exist frontend\dist\index.html ( call "%~f0" build || exit /b 1 )
if not exist backend\venv\Scripts\activate.bat ( echo [formsculpt] backend venv missing, run formsculpt install & exit /b 1 )
call backend\venv\Scripts\activate.bat
set "SVCPORT=%FSC_PORT%"
if not defined SVCPORT set "SVCPORT=8260"
set "FSC_DIST=%~dp0frontend\dist"
echo [formsculpt] serving http://127.0.0.1:%SVCPORT%
cd backend && uvicorn app.main:app --host 127.0.0.1 --port %SVCPORT%
exit /b %errorlevel%
:engine
if /i "%SUB%"=="check" goto enginecheck
if /i "%SUB%"=="wasm" goto enginewasm
if not "%SUB%"=="" if /i not "%SUB%"=="build" goto usage
where cargo >nul 2>nul || ( echo [formsculpt] cargo not found; install Rust from https://rustup.rs & exit /b 1 )
pushd native
cargo build --workspace --release --exclude fsc-py || ( popd & exit /b 1 )
popd
if not exist backend\venv\Scripts\activate.bat ( echo [formsculpt] backend venv missing; the python engine module was not built, run formsculpt install & goto end )
call backend\venv\Scripts\activate.bat
where maturin >nul 2>nul || pip install maturin
pushd native\crates\fsc-py
set "CARGO_TARGET_DIR=%~dp0native\target"
maturin develop --release || ( popd & exit /b 1 )
popd
python -c "import fsc_engine; print('[formsculpt] python engine module', fsc_engine.version())"
goto end
:enginecheck
where cargo >nul 2>nul || ( echo [formsculpt] cargo not found; install Rust from https://rustup.rs & exit /b 1 )
pushd native
cargo check --workspace --all-targets || ( popd & exit /b 1 )
cargo clippy --version >nul 2>nul && ( cargo clippy --workspace --all-targets -- -D warnings || ( popd & exit /b 1 ) )
popd
echo [formsculpt] engine check clean
exit /b 0
:enginewasm
where cargo >nul 2>nul || ( echo [formsculpt] cargo not found; install Rust from https://rustup.rs & exit /b 1 )
pushd native
cargo build -p fsc-wasm --lib --release --target wasm32-unknown-unknown || ( popd & echo [formsculpt] the browser engine did not build; if the wasm32-unknown-unknown target is missing run: rustup target add wasm32-unknown-unknown & exit /b 1 )
popd
if not exist frontend\public\engine mkdir frontend\public\engine
copy /y native\target\wasm32-unknown-unknown\release\fsc_wasm.wasm frontend\public\engine\fsc_engine.wasm >nul || exit /b 1
echo [formsculpt] browser engine at frontend\public\engine\fsc_engine.wasm
exit /b 0
:installdesktop
if not exist backend\venv\Scripts\activate.bat ( echo [formsculpt] backend venv missing, run formsculpt install & exit /b 1 )
call backend\venv\Scripts\activate.bat
pip install -r backend\requirements_desktop.txt || exit /b 1
where cargo >nul 2>nul || ( echo [formsculpt] Rust is not installed. Install it from https://rustup.rs then rerun formsculpt install-desktop & exit /b 1 )
if exist frontend\node_modules\.bin\tauri.cmd goto desktopready
pushd frontend
call npm install || ( popd & exit /b 1 )
popd
:desktopready
for /f "tokens=2" %%V in ('rustc --version') do echo [formsculpt] desktop toolchain ready: rustc %%V
exit /b 0
:freeze
if not exist backend\venv\Scripts\activate.bat ( echo [formsculpt] backend venv missing, run formsculpt install & exit /b 1 )
call backend\venv\Scripts\activate.bat
python -c "import PyInstaller" 2>nul || ( echo [formsculpt] PyInstaller missing, run formsculpt install-desktop & exit /b 1 )
set "TRIPLE=%FSC_TARGET%"
if not defined TRIPLE for /f "tokens=2" %%T in ('rustc -vV 2^>nul ^| findstr /b "host:"') do set "TRIPLE=%%T"
if not defined TRIPLE ( echo [formsculpt] rustc not found; install Rust or set FSC_TARGET & exit /b 1 )
if not exist frontend\dist\index.html ( call "%~f0" build || exit /b 1 )
pushd backend
python -c "import fsc_engine, app; assert fsc_engine.version() == app.__version__" 2>nul
if errorlevel 1 ( popd & call "%~f0" engine || exit /b 1 ) else ( popd )
pushd backend
if exist build\pyinstaller rmdir /s /q build\pyinstaller
if exist dist-engine rmdir /s /q dist-engine
set "FSC_CONSOLE=--noconsole"
if "%FSC_ENGINE_WINDOWED%"=="0" set "FSC_CONSOLE="
pyinstaller --noconfirm --clean --onefile --name formsculpt-engine --distpath dist-engine --workpath build\pyinstaller --specpath build\pyinstaller --add-data "%~dp0frontend\dist;frontend_dist" --add-data "%~dp0integrations\blender\formsculpt_link;integrations/blender/formsculpt_link" --add-data "%~dp0integrations\unreal;integrations/unreal" --add-data "%~dp0integrations\unity;integrations/unity" --add-data "%~dp0plugins;plugins" --collect-submodules app --collect-submodules uvicorn --collect-all fsc_engine --hidden-import app.services.cmd_terminal --hidden-import app.services.claude_code --hidden-import app.services.cmd_chat --hidden-import winpty %FSC_CONSOLE% "%~dp0backend\formsculpt_engine.py" || ( popd & exit /b 1 )
popd
if not exist src-tauri\binaries mkdir src-tauri\binaries
copy /y backend\dist-engine\formsculpt-engine.exe "src-tauri\binaries\formsculpt-engine-%TRIPLE%.exe" >nul || exit /b 1
"src-tauri\binaries\formsculpt-engine-%TRIPLE%.exe" --selfcheck || exit /b 1
echo [formsculpt] sidecar ready: src-tauri\binaries\formsculpt-engine-%TRIPLE%.exe
exit /b 0
:desktop
if /i "%SUB%"=="build" goto desktopbuild
if "%SUB%"=="" goto desktopdev
if /i "%SUB%"=="dev" goto desktopdev
goto usage
:desktopdev
set "TRIPLE=%FSC_TARGET%"
if not defined TRIPLE for /f "tokens=2" %%T in ('rustc -vV 2^>nul ^| findstr /b "host:"') do set "TRIPLE=%%T"
if not defined TRIPLE ( echo [formsculpt] rustc not found; run formsculpt install-desktop & exit /b 1 )
if not exist "src-tauri\binaries\formsculpt-engine-%TRIPLE%.exe" ( call "%~f0" freeze || exit /b 1 )
call frontend\node_modules\.bin\tauri dev %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
:desktopbuild
call "%~f0" build || exit /b 1
call "%~f0" freeze || exit /b 1
if not exist src-tauri\icons\icon.icns call frontend\node_modules\.bin\tauri icon assets\formsculpt_icon.png
call frontend\node_modules\.bin\tauri build %3 %4 %5 %6 %7 %8 %9
if errorlevel 1 exit /b 1
echo [formsculpt] bundles are in src-tauri\target\release\bundle
exit /b 0
:app
call build_app.bat %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
:doctor
set "FAIL=0"
echo [formsculpt] doctor - windows
set "NODEOK=0"
where node >nul 2>nul && node -e "const [m,n]=process.versions.node.split('.').map(Number);process.exit((m===20&&n>=19)||(m===22&&n>=12)||m>=23?0:1)" && set "NODEOK=1"
if "%NODEOK%"=="1" ( for /f "delims=" %%V in ('node -v') do echo   ok     node %%V ) else ( echo   FAIL   node - Node 20.19+ or 22.12+ is required & set "FAIL=1" )
where npm >nul 2>nul && ( echo   ok     npm ) || ( echo   FAIL   npm - install npm & set "FAIL=1" )
set "PYOK=0"
where python >nul 2>nul && python -c "import sys; raise SystemExit(0 if sys.version_info >= (3, 12) else 1)" && set "PYOK=1"
if "%PYOK%"=="1" ( for /f "tokens=2" %%V in ('python -V 2^>^&1') do echo   ok     python %%V ) else ( echo   FAIL   python - Python 3.12+ is required & set "FAIL=1" )
where cargo >nul 2>nul && ( for /f "tokens=2" %%V in ('rustc --version') do echo   ok     rust %%V ) || ( echo   FAIL   rust - the engine needs https://rustup.rs & set "FAIL=1" )
if exist backend\venv\Scripts\activate.bat ( echo   ok     backend venv ) else ( echo   warn   backend venv - run formsculpt install )
if exist frontend\node_modules ( echo   ok     frontend node_modules ) else ( echo   warn   frontend deps - run formsculpt install )
set "I18N=0"
if "%PYOK%"=="1" python -I setup\i18n.py check >nul 2>nul && set "I18N=1"
if "%I18N%"=="1" ( echo   ok     interface translations complete ) else ( echo   warn   interface translations - run formsculpt i18n check; every table should cover strings.json )
set "SVCPORT=%FSC_PORT%"
if not defined SVCPORT set "SVCPORT=8260"
for %%P in (5260 %SVCPORT%) do ( netstat -an | findstr /r /c:":%%P .*LISTENING" >nul && ( echo   warn   port %%P - already in use ) || ( echo   ok     port %%P free ) )
if "%FAIL%"=="0" ( echo [formsculpt] ready ) else ( echo [formsculpt] fix the FAIL lines above )
exit /b %FAIL%
:test
if "%SUB%"=="" goto testall
if /i "%SUB%"=="all" goto testall
if /i "%SUB%"=="engine" goto testengine
if /i "%SUB%"=="service" goto testservice
if /i "%SUB%"=="e2e" goto testwindow
if /i "%SUB%"=="window" goto testwindow
echo [formsculpt] use: formsculpt test [all^|engine^|service^|e2e]
exit /b 1
:testall
call "%~f0" test engine || exit /b 1
call "%~f0" test service || exit /b 1
echo [formsculpt] engine and service tests passed; formsculpt test e2e runs the window smoke test against a booted window
exit /b 0
:testengine
where cargo >nul 2>nul || ( echo [formsculpt] cargo not found; install Rust from https://rustup.rs & exit /b 1 )
pushd native
cargo test --workspace || ( popd & exit /b 1 )
popd
if not exist frontend\public\engine\fsc_engine.wasm ( echo [formsculpt] the browser engine is not built, see formsculpt engine wasm; its smoke test was skipped & exit /b 0 )
where node >nul 2>nul || ( echo [formsculpt] node not found; the browser engine's smoke test was skipped & exit /b 0 )
node frontend\e2e\wasm_smoke.mjs frontend\public\engine\fsc_engine.wasm
exit /b %errorlevel%
:testservice
if not exist backend\venv\Scripts\activate.bat ( echo [formsculpt] backend venv missing, run formsculpt install & exit /b 1 )
call backend\venv\Scripts\activate.bat
python -c "import pytest, httpx" 2>nul || pip install -r backend\requirements_test.txt || exit /b 1
python -c "import fsc_engine" 2>nul || ( echo [formsculpt] the engine module is not built; run formsculpt engine & exit /b 1 )
pushd backend
python -m pytest -q -p no:warnings tests
set "RC=%errorlevel%"
popd
exit /b %RC%
:testwindow
python -c "import playwright" 2>nul || ( echo [formsculpt] Python Playwright not found; pip install playwright and run playwright install chromium & exit /b 1 )
set "WINDOWURL=%FSC_WINDOW_URL%"
if not defined WINDOWURL set "WINDOWURL=http://localhost:5260/"
set "SERVICEURL=%FSC_SERVICE_URL%"
if not defined SERVICEURL set "SERVICEURL=http://127.0.0.1:8260"
python frontend\e2e\smoke.py "%WINDOWURL%" "%SERVICEURL%"
exit /b %errorlevel%
:bench
where cargo >nul 2>nul || ( echo [formsculpt] cargo not found; install Rust from https://rustup.rs & exit /b 1 )
set "LEVEL=%SUB%"
if "%LEVEL%"=="" set "LEVEL=8"
pushd native
cargo build --release -q -p fsc-core --bin fsc-bench || ( popd & exit /b 1 )
popd
if not exist bench-results mkdir bench-results
for /f %%S in ('powershell -NoProfile -Command "[DateTime]::UtcNow.ToString('yyyyMMddTHHmmssZ')"') do set "STAMP=%%S"
native\target\release\fsc-bench.exe %LEVEL% > "bench-results\engine-%STAMP%.json" || exit /b 1
type "bench-results\engine-%STAMP%.json"
echo [formsculpt] written bench-results\engine-%STAMP%.json
exit /b 0
:mcp
python setup\mcp_stdio.py %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
:script
set "FSC_CWD=%CALLER_DIR%"
set "PY=python"
if exist backend\venv\Scripts\python.exe set "PY=backend\venv\Scripts\python.exe"
"%PY%" setup\run_script.py %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
:i18n
python -I setup\i18n.py %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
:agents
python setup\agents.py %2 %3 %4 %5 %6 %7 %8 %9
exit /b %errorlevel%
:clean
for %%D in (frontend\dist backend\build backend\dist-engine src-tauri\target src-tauri\gen native\target) do if exist "%%D" rmdir /s /q "%%D"
for /d /r backend %%D in (__pycache__) do if exist "%%D" rmdir /s /q "%%D"
if exist src-tauri\binaries del /q src-tauri\binaries\formsculpt-engine-* 2>nul
if /i "%SUB%"=="--all" for %%D in (frontend\node_modules backend\venv) do if exist "%%D" rmdir /s /q "%%D"
echo [formsculpt] clean
exit /b 0
:usage
echo usage: formsculpt [command]
echo   boot ^| install ^| install-desktop ^| backend ^| frontend ^| build ^| serve ^| freeze
echo   engine [build^|check^|wasm] ^| desktop dev^|build ^| app [flags] ^| doctor ^| test [all^|engine^|service^|e2e] ^| bench [level]
echo   mcp ^| script FILE [JSON] ^| i18n extract^|check ^| agents sync^|check^|install ^| clean [--all] ^| help
:end
endlocal
