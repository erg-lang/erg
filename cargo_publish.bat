@echo off

if %~dp0 == C:%homepath%\GitHub\erg\ (
    cd crates/erg_common
    echo publish erg_common ...
    cargo publish
    rem from cargo 1.66 timeout is not needed
    echo publish erg_proc_macro ...
    cd ../erg_proc_macro
    cargo publish
    cd ../erg_parser
    echo publish erg_parser ...
    cargo publish
    cd ../erg_compiler
    echo publish erg_compiler ...
    cargo publish
    cd ../els
    echo publish els ...
    cargo publish
    cd ../../
    cargo publish
    echo completed
) else (
    echo Use this command in the project root
)
