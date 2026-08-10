# Makefile for tree-sitter-rock

.PHONY: all build install test clean generate

all: build

generate:
	tree-sitter generate

build: generate
	tree-sitter build

test: build
	tree-sitter test

install: build
	npm install

clean:
	rm -rf src/parser.c
	rm -rf build/
