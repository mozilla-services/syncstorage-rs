"""Utility functions for load testing."""

import base64


def b64encode(data: bytes) -> str:
    """Encode bytes to base64 ASCII string.

    Args:
        data: Bytes to encode.

    Returns:
        str: Base64-encoded ASCII string.

    """
    return base64.b64encode(data).decode("ascii")
