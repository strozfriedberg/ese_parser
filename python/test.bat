virtualenv test
call test/Scripts/activate.bat
pip install --force-reinstall .\target\wheels\ese_parser-0.2.0-cp310-none-win_amd64.whl
python py\test.py
if %ERRORLEVEL% neq 0 (exit /b 1)
call test/Scripts/deactivate.bat