@echo off

if %~dp0 == C:%homepath%\GitHub\erg\ (
    cd compiler/erg_common
    echo publish erg_common ...
    cargo publish
    rem from cargo 1.66 timeout is not needed
    rem timeout 12
    cd ../erg_parser
    echo publish erg_parser ...
    cargo publish
    rem timeout 12
    cd ../erg_compiler
    echo publish erg_compiler ...
    cargo publish
    rem timeout 12
    cd ../els
    echo publish els ...
    cargo publish
    cd ../../
    cargo publish
    echo completed
) else (
    echo Use this command in the project root
)
