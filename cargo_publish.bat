@echo off

if %~dp0 == C:%homepath%\GitHub\erg\ (
    cd compiler/erg_common
    echo publish erg_common ...
    cargo publish
    timeout 10
    cd ../erg_parser
    echo publish erg_parser ...
    cargo publish
    timeout 10
    cd ../erg_compiler
    echo publish erg_compiler ...
    cargo publish
    timeout 10
    cd ../../
    cargo publish
    echo completed
) else (
    echo Use this command in the project root
)
