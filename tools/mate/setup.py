from setuptools import setup, find_packages
import pip

with open('README.md') as f:
    readme = f.read()

install_reqs = pip.req.parse_requirements(
    'requirements.txt', session=pip.download.PipSession())

reqs = [str(ir.req) for ir in install_reqs]

setup(
    name='mate',
    version='1.0.0',
    description='Debug tools for Nao',
    long_description=readme,
    url='https://hulks.de',
    license=license,
    install_requires=reqs,
    packages=find_packages())
