#!/bin/bash
# for this to work instal cross like this: 
# cargo install cross

cross build --release --target armv7-unknown-linux-gnueabihf
