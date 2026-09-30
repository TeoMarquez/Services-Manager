@echo off
cd /d "%~dp0"
cargo build --release --locked --bin api
