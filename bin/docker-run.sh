#!/bin/sh

TOP=$(git rev-parse --show-toplevel)
docker run -it --mount type=bind,source=${TOP},target=/cs3210 cs3210 /bin/bash
