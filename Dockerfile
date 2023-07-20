FROM rustlang/rust:nightly

USER root
ENV USER root
ENV SCRIPT_FOLDER scripts
ENV LOG_LEVEL debug

# Copy source into container
WORKDIR /usr/src/rusty-hooks
COPY . .

# Build the application binary
RUN cargo build --release

# Print runtime value of required args
RUN echo ${SCRIPT_FOLDER}
RUN echo ${LOG_LEVEL}

CMD /usr/src/rusty-hooks/target/release/rusty-hooks --script-folder ${SCRIPT_FOLDER} --log-level ${LOG_LEVEL}

# docker run -v /music:/home/syncthing/music -v /movies:/media/wd_red/wd_red_sync_folder/movies rusty-hooks:latest
# docker run --rm -it -v /music:/home/syncthing/music -v /movies:/media/wd_red/wd_red_sync_folder/movies rusty-hooks:latest
