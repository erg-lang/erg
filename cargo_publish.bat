@echo off

if %~dp0 == %homepath%\sbym8\GitHub\erg\ (
    cd compiler/erg_common
    echo publish erg_common ...
    cargo publish
    timeout 5
    cd ../erg_parser
    echo publish erg_parser ...
    cargo publish
    timeout 5
    cd ../erg_compiler
    echo publish erg_compiler ...
    cargo publish
    cd ../../
    echo completed
) else (
    echo Use this command in the project root
)
