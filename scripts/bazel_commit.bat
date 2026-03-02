@echo off
powershell -ExecutionPolicy Bypass -File %~dp0\bazel_commit.ps1 %*
