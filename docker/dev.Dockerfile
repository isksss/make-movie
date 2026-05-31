FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive
ARG USER_ID=1000
ARG GROUP_ID=1000
ARG NODE_MAJOR=26
ARG PNPM_VERSION=10.24.0

ENV APP_USER=ubuntu
ENV APP_HOME=/home/ubuntu
ENV CARGO_HOME=/home/ubuntu/.cargo
ENV RUSTUP_HOME=/home/ubuntu/.rustup
ENV PATH=/home/ubuntu/.cargo/bin:/home/ubuntu/.local/share/pnpm:/usr/local/go/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ENV PNPM_HOME=/home/ubuntu/.local/share/pnpm
ENV PLAYWRIGHT_BROWSERS_PATH=/home/ubuntu/.cache/ms-playwright

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
      bash \
      build-essential \
      ca-certificates \
      curl \
      file \
      ffmpeg \
      fonts-liberation \
      git \
      gnupg \
      golang-go \
      libappindicator3-dev \
      libasound2t64 \
      libatk-bridge2.0-0 \
      libatk1.0-0 \
      libatspi2.0-0 \
      libcairo2 \
      libcups2 \
      libdbus-1-3 \
      libdrm2 \
      libfontconfig-dev \
      libfreetype-dev \
      libgbm1 \
      libglib2.0-0 \
      libgtk-3-0 \
      libnspr4 \
      libnss3 \
      libpango-1.0-0 \
      librsvg2-dev \
      libssl-dev \
      libwebkit2gtk-4.1-dev \
      libx11-6 \
      libx11-xcb1 \
      libxcb1 \
      libxcomposite1 \
      libxdamage1 \
      libxext6 \
      libxfixes3 \
      libxkbcommon0 \
      libxrandr2 \
      patchelf \
      pkg-config \
      wget \
      xvfb \
    && install -d -m 0755 /etc/apt/keyrings \
    && curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key \
      | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg \
    && printf 'deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_%s.x nodistro main\n' "$NODE_MAJOR" \
      > /etc/apt/sources.list.d/nodesource.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends nodejs \
    && corepack enable \
    && corepack prepare "pnpm@${PNPM_VERSION}" --activate \
    && rm -rf /var/lib/apt/lists/*

RUN if id -u ubuntu >/dev/null 2>&1; then \
      groupmod --non-unique --gid "$GROUP_ID" ubuntu \
      && usermod --non-unique --uid "$USER_ID" --gid "$GROUP_ID" --home /home/ubuntu ubuntu; \
    else \
      groupadd --non-unique --gid "$GROUP_ID" ubuntu \
      && useradd --non-unique --uid "$USER_ID" --gid ubuntu --create-home --home-dir /home/ubuntu --shell /bin/bash ubuntu; \
    fi

USER ubuntu

RUN corepack prepare "pnpm@${PNPM_VERSION}" --activate
RUN corepack pnpm config set store-dir /home/ubuntu/.local/share/pnpm/store

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain stable --profile minimal --component rustfmt --component clippy

USER root

COPY docker/entrypoint.sh /usr/local/bin/mm-docker-entrypoint

RUN chmod 0755 /usr/local/bin/mm-docker-entrypoint \
    && install -d -o ubuntu -g ubuntu /workspace /home/ubuntu/.cache /home/ubuntu/.local/share/pnpm \
    && chown -R ubuntu:ubuntu /home/ubuntu \
    && install -d -m 1777 /tmp/.X11-unix

USER ubuntu
WORKDIR /workspace

ENTRYPOINT ["mm-docker-entrypoint"]
CMD ["bash"]
