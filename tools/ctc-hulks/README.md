# Building the HULKs CrossToolchain

## Known issues

* This toolchain is nao v6 only!


## Instructions for doing it manually

0. Required packages  : build-essential automake autoconf gperf bison flex texinfo libtool libtool-bin gawk libncurses5-dev unzip cmake libexpat-dev python2.7-dev nasm help2man ninja
   Required variables : LD_LIBRARY_PATH and CPATH need to be empty
   Note: ubuntu users might have a different ninja package preinstalled which is not the required one, if errors occur do `sudo apt-get purge ninja` and `sudo apt-get install ninja-build`
1. run script `1-setup`.
2. run script `2-build-toolchain`.
3. Check if the toolchain was correctly built (look into the x-tools folder if the toolchain exists [bin/gcc,bin/ld,etc])
4. run script `3-build-libs`.
5. run script `4-install`.
6. Now there should be a ctc-linux**-hulks-** folder.
7. Now there should be a sysroot and ctc-linux**-hulks-**.tar.gz.


## Instructions for using Docker

The Dockerfile sets up an image which is able to build our toolchain. Anyway,
you may want to use the docker image. The image has
all dependencies included and is willing to build the toolchain. You can find
the image (`ctc_full.tar`) in our tools folder on the bighulk.

`0-clean`, `1-setup-ctng`, `2-setup-libs`, `3-build-toolchain`, `4-build-libs`, `5-install`
were already executed inside the image.

Go inside `/ctc-hulks` to find the relevant files.


### Import Docker Image

```
cat ctc_full.tar | docker import - ctc
```


### Run Docker Image

```
docker run -it ctc zsh
```



### (Export Docker Container)

```
docker export -o ctc.tar.bz2 <container id>
```

