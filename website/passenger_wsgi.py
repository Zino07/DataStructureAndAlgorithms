"""Phusion Passenger WSGI entrypoint for Octenium cPanel ("Setup Python App").

cPanel's CloudLinux Python Selector loads ``passenger_wsgi.py`` by default and
expects a WSGI callable named ``application``.
"""
import os
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from wsgiapp import application  # noqa: E402

app = application
