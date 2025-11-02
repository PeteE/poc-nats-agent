# Tiltfile for NATS Agent POC

# Allow k8s contexts (minikube)
allow_k8s_contexts('minikube')

# Load Helm extension
load('ext://helm_remote', 'helm_remote')

# Deploy cert-manager
helm_remote('cert-manager',
    repo_url='https://charts.jetstack.io',
    namespace='cert-manager',
    create_namespace=True,
    set=[
        'crds.enabled=true',
    ],
    version='v1.19.0',
)

# Deploy OpenTelemetry Operator
helm_remote('opentelemetry-operator',
    repo_url='https://open-telemetry.github.io/opentelemetry-helm-charts',
    namespace='otel-system',
    create_namespace=True,
    set=[
        'manager.collectorImage.repository=otel/opentelemetry-collector-k8s',
    ]
)

# # Deploy OpenTelemetry Collector
k8s_yaml('k8s/otel-collector.yaml')
# k8s_resource(
#     'otel-collector',
#     port_forwards=['4317:4317', '4318:4318'],  # OTLP gRPC and HTTP
#     labels=['observability'],
#     resource_deps=['opentelemetry-operator']
# )

# # Deploy NATS
# helm_remote('nats',
#     repo_url='https://nats-io.github.io/k8s/helm/charts/',
#     namespace='default',
#     set=[
#         'nats.jetstream.enabled=true',
#     ]
# )

# k8s_resource(
#     'nats',
#     port_forwards='4222:4222',  # NATS client port
#     labels=['infrastructure']
# )

# Future: Add Rust agent container build/deploy here
# docker_build('nats-agent', '.')
# k8s_yaml('k8s/agent.yaml')
