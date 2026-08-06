FROM amazonlinux:2023

RUN mkdir -p /build/src

RUN dnf update -y

RUN dnf install -y \
    gcc gcc-c++ make \
    perl git jq \
    openssl-devel openssl \
    postgresql-devel \
    mariadb-connector-c-devel \
    zip tar

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN /root/.cargo/bin/cargo --version
RUN /root/.cargo/bin/rustc --version


# Build environment setting
WORKDIR /build
ENV FEATURES="sqlite,postgresql"
ENV CARGOFLAGS="--release"
ENV PATH=/root/.cargo/bin:/usr/sbin:/usr/bin:/sbin:/bin

COPY build.sh /build.sh
RUN chmod +x /build.sh

ENTRYPOINT ["/build.sh"]
