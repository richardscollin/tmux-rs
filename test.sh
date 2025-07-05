#!/bin/bash

cargo build

cp target/debug/tmux-rs tests/tmux

FAILURES=0

cd tests/suite
for file in *.sh; do
  printf "RUNNING $file\n\e[1;33m"
  if $(/bin/sh $file); then
    printf "\e[32mSUCCESS $file\e[0m\n"
  else
    printf "\e[31mFAILURE $file\e[0m\n"
    ((FAILURES++))
  fi
done

if [ "$FAILURES" -gt "0" ]; then
  printf "\e[31mFAILURE\e[0m: $FAILURES tests\n"
else
  printf "\e[32mSUCCESS\e[0m\n"
fi
