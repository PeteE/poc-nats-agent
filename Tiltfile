# Tiltfile for NATS Agent POC

# Allow k8s contexts (minikube)
allow_k8s_contexts('minikube')

# Configure default registry to use in-cluster registry
default_registry(
    'reg.wheat-dn42.net'
)

# Load Helm extensions
load('ext://helm_remote', 'helm_remote')
load('ext://helm_resource', 'helm_resource', 'helm_repo')

# Deploy cert-manager
helm_remote('cert-manager',
    repo_url='https://charts.jetstack.io',
    namespace='cert-manager',
    create_namespace=True,
    set=[
        'crds.enabled=true',
    ],
    version='v1.19.0',
    # labels=['infrastructure'],
)

load('ext://secret', 'secret_from_dict')
k8s_yaml(
    secret_from_dict(
        name='cloudflare-api-token',
        namespace='cert-manager',
        inputs={'api-token': os.getenv('CLOUDFLARE_API_TOKEN')},
    )
)

# Deploy Letsencrypt Cluster Issuer
k8s_yaml('k8s/manifests/cert-manager.yaml')

# Deploy external-dns
helm_remote('external-dns',
    repo_url='https://kubernetes-sigs.github.io/external-dns',
    namespace='cert-manager',
    values=[
        'k8s/values/dns.yaml',
    ],
    version='1.19.0',
)

# Deploy Gateway API crds
k8s_yaml('k8s/manifests/gateway-api-crds.yaml')

# kgateway CRDS
helm_remote('kgateway-crds',
    repo_url='oci://cr.kgateway.dev/kgateway-dev/charts',
    namespace='kgateway-system',
    create_namespace=True,
    version='v2.1.0',
)

# kgateway ingress
helm_remote('kgateway',
    repo_url='oci://cr.kgateway.dev/kgateway-dev/charts',
    namespace='kgateway-system',
    version='v2.1.0',
)

# Deploy Wildcard Certificate
k8s_yaml('k8s/manifests/ingress-wildcard-cert.yaml')

# Deploy Ingress conroller
k8s_yaml('k8s/manifests/gateway.yaml')

# Deploy OpenTelemetry Operator
helm_remote('opentelemetry-operator',
    repo_url='https://open-telemetry.github.io/opentelemetry-helm-charts',
    namespace='otel-system',
    create_namespace=True,
    set=[
        'manager.collectorImage.repository=otel/opentelemetry-collector-k8s',
    ],
    # labels=['infrastructure'],
)

# Deploy OpenTelemetry Collector
k8s_yaml('k8s/otel-collector.yaml')

# k8s_resource(
#     'opentelemetry-operator',
#     port_forwards=[],
#     labels=['monitoring']
# )

# Deploy Prometheus
helm_remote('kube-prometheus-stack',
    repo_url='https://prometheus-community.github.io/helm-charts',
    namespace='monitoring',
    create_namespace=True,
    install_crds=True,
    set=[
      'alertmanager.enabled=false',
      'grafana.enabled=false',
      'prometheus.enabled=true',
      'prometheusOperator.enabled=true',
      'prometheus.prometheusSpec.enableOTLPReceiver=true',
      'crds.enabled=false',
      'kubernetesServiceMonitors.enabled=false',
      'kubeApiServer.enabled=false',
      'kubelet.enabled=false',
      'defaultRules.create=false',
      'kubeControllerManager.enabled=false',
      'coreDns.enabled=false',
      'kubeEtcd.enabled=false',
      'kubeScheduler.enabled=false',
      'kubeProxy.enabled=false',
      'kubeStateMetrics.enabled=false',
      'nodeExporter.enabled=false',
    ],
    # labels=['infrastructure', 'observability'],
)

# k8s_resource(
#     'kube-prometheus-stack-operator',
#     port_forwards='9090:9090',
#     labels=['monitoring']
# )

# Deploy NATS
helm_remote('nats',
    repo_url='https://nats-io.github.io/k8s/helm/charts/',
    namespace='nats-io',
    create_namespace=True,
    set=[
        'config.jetstream.enabled=true',
    ],
)

k8s_resource(
    'nats',
    port_forwards='4222:4222',  # NATS client port
    labels=['infrastructure','nats'],
)

# Build NATS agent container using Nix and push to registry
custom_build(
    'reg.wheat-dn42.net/poc-nats-agent',
    'nix build .#nats-agent-container && ./result | gzip --fast | skopeo copy docker-archive:/dev/stdin docker://reg.wheat-dn42.net/poc-nats-agent:latest && nix build .#producer',
    deps=['src', 'Cargo.toml', 'Cargo.lock', 'flake.nix'],
    tag='latest',
    skips_local_docker=True,
)

# Deploy NATS agent using Helm
helm_resource(
    name='nats-agent-helm',
    chart='./k8s/nats-agent',
    namespace='default',
    release_name='poc-nats-agent',
    image_deps=[('reg.wheat-dn42.net/poc-nats-agent')],
    image_keys=[('image.registry', 'image.repository', 'image.tag')],
    resource_deps=['nats'],
    port_forwards='8080:8080',
    labels=['nats-agent'],
    flags=[
        '--set', 'nats.url=nats://nats.nats-io.svc.cluster.local:4222',
        '--set', 'nats.streamName=events',
        '--set', 'nats.subjects=events.>',
        '--set', 'nats.consumerName=nats-agent',
        '--set', 'nats.batchSize=10',
        '--set', 'opentelemetry.enabled=true',
        '--set', 'opentelemetry.serviceName=nats-agent',
        '--set', 'opentelemetry.endpoint=http://otel-collector-collector.otel-system.svc.cluster.local:4317',
    ]
)
