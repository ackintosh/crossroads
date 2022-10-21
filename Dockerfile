FROM rust:1.64

WORKDIR /app

RUN apt-get update \
  && apt-get upgrade -y \
  && apt-get install -y -q \
  iptables \
  iproute2 \
  ethtool \
  iputils-ping \
  arping

COPY root.bashrc /root/.bashrc

