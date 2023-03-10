docker run --rm -it `
    --add-host=host.docker.internal:host-gateway `
    -p 9090:9090 `
    -v ${PWD}/dev/prometheus/prometheus.yml:/etc/prometheus/prometheus.yml `
    prom/prometheus