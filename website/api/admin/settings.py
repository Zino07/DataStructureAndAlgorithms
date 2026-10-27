"""Vercel route — binds the shared Noor WSGI app (see /wsgiapp.py)."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))

from wsgiapp import application as app

