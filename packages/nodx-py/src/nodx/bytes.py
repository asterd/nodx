import base64
import hashlib


def sha256_base64_url(input_):
    data = input_.encode("utf-8") if isinstance(input_, str) else input_
    digest = hashlib.sha256(data).digest()
    encoded = base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")
    return "sha256-" + encoded
