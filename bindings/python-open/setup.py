from setuptools import setup
from setuptools.dist import Distribution


class BinaryDistribution(Distribution):
    """Force platform-specific wheel tags (not py3-none-any)."""

    def has_ext_modules(self):
        return True


setup(distclass=BinaryDistribution)
