@echo off
chcp 65001 >nul
setlocal
title Black Launcher

cd /d "%~dp0"

echo ========================================
echo          BLACK LAUNCHER
echo ========================================
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo [ОШИБКА] Rust/Cargo не найден.
    echo Установи Rust с сайта https://rustup.rs/
    echo После установки перезапусти это окно.
    pause
    exit /b 1
)

if not exist "Cargo.toml" (
    echo [ОШИБКА] Cargo.toml не найден.
    echo Положи этот BAT-файл в корневую папку проекта,
    echo рядом с Cargo.toml.
    pause
    exit /b 1
)

echo Запуск лаунчера...
echo При первом запуске Cargo может скачать зависимости и собрать проект.
echo Последующие запуски будут быстрее.
echo.

cargo run --release
set "EXIT_CODE=%ERRORLEVEL%"

if not "%EXIT_CODE%"=="0" (
    echo.
    echo [ОШИБКА] Лаунчер завершился с кодом %EXIT_CODE%.
    echo Проверь сообщения выше.
    pause
)

exit /b %EXIT_CODE%
