#!/usr/bin/env python3
"""Local entrypoint for the Noor backend.

The application lives in ``noor_app.py`` so Vercel does not mistake this
project for a single WSGI app: deployment serves the static storefront from the
CDN and only ``/api/*`` through the Python functions. Keep using this wrapper
for local runs and ``./restart.sh``.
"""
from noor_app import main

if __name__ == "__main__":
    main()
