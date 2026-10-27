#!/usr/bin/env python3
"""Local entrypoint for the Noor backend.

The application lives in ``noor_app.py``; on cPanel it runs through
``passenger_wsgi.py``. Use this wrapper for local runs and ``./restart.sh``.
"""
from noor_app import main

if __name__ == "__main__":
    main()
