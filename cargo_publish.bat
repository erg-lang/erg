@echo off

if %~dp0 == C:%homepath%\GitHub\erg\ (
    echo publish erg_proc_macros ...
    cd crates/erg_proc_macros
    cargo publish
    echo publish erg_common ...
    cd ../erg_common
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
