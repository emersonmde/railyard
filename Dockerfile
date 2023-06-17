# Use an official Rust runtime as a parent image
FROM rust:1.57

# Set the working directory in the container to /app
WORKDIR /usr/src/railyard

# Copy the current directory contents into the container
# COPY . /usr/src/railyard
COPY . .

# Install openssl
# RUN apt-get update && \
    # apt-get install -y openssl libssl-dev

# Generate a self-signed certificate
# RUN openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes -subj '/CN=localhost'

# Build the application
# RUN cargo build --release

RUN cargo install --path .

# Make port 8080 available to the world outside this container
EXPOSE 5000

# Run the app when the container launches
# CMD ["cargo", "run", "--release"]
# CMD ["./target/release/railyard"]

CMD ["railyard"]
