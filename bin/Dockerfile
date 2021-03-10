FROM ubuntu:20.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update
RUN apt-get install -y -q tzdata
RUN apt-get install -y -q sudo
RUN echo "%sudo ALL=(ALL:ALL) NOPASSWD:ALL" >> /etc/sudoers
RUN apt-get install -y -q build-essential python3 wget curl tar screen clang-8 lld-8 linux-image-extra-virtual

RUN useradd --uid 1004 --user-group --create-home user
RUN usermod -aG sudo user

USER user

WORKDIR /home/user
COPY . bin

RUN sudo apt-get install -y -q git
RUN git config --global user.email "you@example.com"
RUN git config --global user.name "Your Name"

RUN bin/setup.sh
