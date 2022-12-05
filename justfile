# simple project just file
# see https://github.com/casey/just
# Find instructions on how to set up just locally at: https://just.systems/man/en/

set dotenv-load

alias c := full-check
alias u := update
#alias d := build-docker
#alias drl := docker-run-local

default:
  just --list

full-check:
  cargo fmt
  cargo check
  cargo clippy

update:
  cargo upgrade --workspace
  cargo update

#build-docker:
#  cargo test
#  docker build --tag bankaccount --file Dockerfile

