# Tiltfile for NATS Agent POC

# Allow k8s contexts (minikube)
allow_k8s_contexts('minikube')

# Configure default registry to use in-cluster registry
default_registry(
    'reg.wheat-dn42.net'
)

# Tilt's default 30s upsert timeout is shorter than the `helm upgrade
# --install` calls in this file can legitimately take (e.g. cert-manager's
# startupapicheck hook, given how many CRDs/webhook-configs cainjector has
# to churn through in this cluster before it's satisfied). Without this,
# Tilt kills the apply before Helm's own retry window ever completes.
update_settings(k8s_upsert_timeout_secs=360)

# Load Helm extensions
load('ext://namespace', 'namespace_create', 'namespace_inject')
load('ext://helm_resource', 'helm_resource', 'helm_repo')
load('ext://secret', 'secret_from_dict', 'secret_yaml_docker_registry')

################## cert-manager
# helm_resource runs real `helm upgrade --install`, so Helm owns CRD
# create/update itself (no k8s_yaml/last-applied-configuration involved).
# helm_remote instead rendered CRDs to YAML and let Tilt's own apply engine
# manage them, which grows the last-applied-configuration annotation past the
# 262144 byte limit on repeated re-applies and fights cert-manager's own
# finalizer-based deletion of its CRs.
helm_repo('jetstack', 'https://charts.jetstack.io')
helm_resource(
    name='cert-manager',
    chart='jetstack/cert-manager',
    namespace='cert-manager',
    release_name='cert-manager',
    resource_deps=['jetstack'],
    flags=[
        '--create-namespace',
        '--version', 'v1.19.0',
        '--set', 'crds.enabled=true',
        # cainjector has to churn through every CRD/webhook-config in the
        # cluster (kgateway, otel-operator, prometheus-operator, ...) before
        # it patches cert-manager-webhook's own CA bundle. That first pass
        # regularly takes >1m here, past the chart's default startupapicheck
        # timeout, failing the install even though cert-manager itself is fine.
        '--set', 'startupapicheck.timeout=5m',
    ],
    labels=['infrastructure'],
)
k8s_yaml(
    secret_from_dict(
        name='cloudflare-api-token',
        namespace='cert-manager',
        inputs={'api-token': os.getenv('CLOUDFLARE_API_TOKEN')},
    )
)
# Deploy Letsencrypt Cluster Issuer
k8s_yaml('k8s/manifests/cert-manager.yaml')
k8s_resource(
    new_name='letsencrypt-issuer',
    objects=['letsencrypt:clusterissuer'],
    resource_deps=['cert-manager'],
)
##################


################## external-dns
namespace_create('external-dns')
k8s_yaml(
    secret_from_dict(
        name='cloudflare-api-token',
        namespace='external-dns',
        inputs={'api-token': os.getenv('CLOUDFLARE_API_TOKEN')},
    )
)
helm_repo('external-dns', 'https://kubernetes-sigs.github.io/external-dns',
    resource_name='external-dns-repo')
helm_resource(
    name='external-dns',
    chart='external-dns/external-dns',
    namespace='external-dns',
    release_name='external-dns',
    resource_deps=['external-dns-repo', 'cert-manager'],
    flags=[
        '--version', '1.19.0',
        '-f', 'k8s/values/dns.yaml',
    ],
    deps=['k8s/values/dns.yaml'],
)
##################

################## Cluster Ingress - kgateway
# Deploy Gateway API crds
k8s_yaml('k8s/manifests/gateway-api-crds.yaml')
# kgateway CRDS
# helm_resource, not helm_remote: helm_remote hands the rendered YAML to
# Tilt's apply engine, which delete+recreates objects on its retry cycles.
# That regenerates the ServiceAccount UID out from under running pods, whose
# projected tokens pin the old UID — every API call then 401s with
# "service account UID ... does not match claim". Real Helm updates in place
# and keeps the UID stable.
helm_resource(
    name='kgateway-crds',
    chart='oci://cr.kgateway.dev/kgateway-dev/charts/kgateway-crds',
    namespace='kgateway-system',
    release_name='kgateway-crds',
    flags=[
        '--create-namespace',
        '--version', 'v2.1.0',
    ],
)
# kgateway ingress
helm_resource(
    name='kgateway',
    chart='oci://cr.kgateway.dev/kgateway-dev/charts/kgateway',
    namespace='kgateway-system',
    release_name='kgateway',
    resource_deps=['kgateway-crds'],
    flags=[
        '--create-namespace',
        '--version', 'v2.1.0',
    ],
)
# Deploy Wildcard Certificate
k8s_yaml('k8s/manifests/ingress-wildcard-cert.yaml')

# Deploy Ingress conroller
k8s_yaml('k8s/manifests/gateway.yaml')
k8s_yaml('k8s/manifests/httproute-poc-nats.yaml')
##################

################## Prometheus stack
# helm_resource (real `helm upgrade --install`) instead of helm_remote so Helm
# manages the prometheus-operator CRDs itself — helm_remote's rendered-YAML
# path pushed CRDs through Tilt's k8s_yaml apply engine, whose
# last-applied-configuration annotation blew past the 262144 byte limit on
# alertmanagerconfigs.monitoring.coreos.com.
helm_repo('prometheus-community', 'https://prometheus-community.github.io/helm-charts')
helm_resource(
    name='kube-prometheus-stack',
    chart='prometheus-community/kube-prometheus-stack',
    namespace='monitoring',
    release_name='kube-prometheus-stack',
    resource_deps=['prometheus-community'],
    flags=[
      '--create-namespace',
      '--set', 'alertmanager.enabled=false',
      '--set', 'grafana.enabled=true',
      '--set', 'prometheus.enabled=true',
      '--set', 'prometheusOperator.enabled=true',
      '--set', 'prometheus.prometheusSpec.enableOTLPReceiver=true',
      '--set', 'crds.enabled=true',
      '--set', 'kubernetesServiceMonitors.enabled=false',
      '--set', 'kubeApiServer.enabled=false',
      '--set', 'kubelet.enabled=false',
      '--set', 'defaultRules.create=false',
      '--set', 'kubeControllerManager.enabled=false',
      '--set', 'coreDns.enabled=false',
      '--set', 'kubeEtcd.enabled=false',
      '--set', 'kubeScheduler.enabled=false',
      '--set', 'kubeProxy.enabled=false',
      '--set', 'kubeStateMetrics.enabled=false',
      '--set', 'nodeExporter.enabled=false',
    ],
)
##################

################## OpenTelemetry Operator
# helm_remote's own CRD-install step only picks up static (non-templated)
# CRD YAML — this chart's CRDs use Helm templating for labels/conditionals
# (gated behind crds.create, default true), so they were silently never
# applied and the operator crash-loops on "missing OpenTelemetryCollector
# CRDs". helm_resource runs a real `helm upgrade --install` so Helm installs
# the CRDs itself, same fix as cert-manager/kube-prometheus-stack above.
helm_repo('open-telemetry', 'https://open-telemetry.github.io/opentelemetry-helm-charts')
helm_resource(
    name='opentelemetry-operator',
    chart='open-telemetry/opentelemetry-operator',
    namespace='otel-system',
    release_name='opentelemetry-operator',
    resource_deps=['open-telemetry'],
    flags=[
        '--create-namespace',
        '--set', 'manager.collectorImage.repository=otel/opentelemetry-collector-k8s',
    ],
)
# Deploy OpenTelemetry Collector
k8s_yaml('k8s/otel-collector.yaml')
##################

################### NATS Server
# helm_resource for the same reason as the charts above: helm_remote lets
# Tilt's apply engine delete+recreate objects on retry, which rotates the
# ServiceAccount UID and 401s any pod still holding a token for the old one.
helm_repo('nats-io', 'https://nats-io.github.io/k8s/helm/charts/')
helm_resource(
    name='nats',
    chart='nats-io/nats',
    namespace='nats-io',
    release_name='nats',
    resource_deps=['nats-io'],
    flags=[
        '--create-namespace',
        '--set', 'config.jetstream.enabled=true',
        # chart default is 10Gi; this POC only ever holds test events
        '--set', 'config.jetstream.fileStore.pvc.size=1Gi',
    ],
    port_forwards='4222:4222',  # NATS client port
    labels=['infrastructure','nats'],
)
###################
# namespace_create('clickhouse')
# k8s_yaml(
#     secret_from_dict(
#         name='clickhouse-creds',
#         namespace='clickhouse',
#         inputs={
#           'user': os.getenv('CLICKHOUSE_USER'),
#           'password': os.getenv('CLICKHOUSE_PASSWORD'),
#         },
#     )
# )
# helm_remote('clickhouse',
#     repo_url='https://helm.altinity.com',
#     namespace='clickhouse',
#     values=[
#         'k8s/values/clickhouse.yaml',
#     ],
# )

################### Build container using Nix and push to registry
custom_build(
    'reg.wheat-dn42.net/poc-nats-agent',
    # --password-stdin keeps the password out of argv and the Tilt log.
    'printf %s "$REGISTRY_PASSWORD" | skopeo login reg.wheat-dn42.net --username "$REGISTRY_USER" --password-stdin && ' +
    'nix build .#nats-agent-container && ./result | gzip --fast | skopeo copy docker-archive:/dev/stdin docker://reg.wheat-dn42.net/poc-nats-agent:latest && nix build .#producer',
    deps=['src', 'Cargo.toml', 'Cargo.lock', 'flake.nix'],
    tag='latest',
    skips_local_docker=True,
)

################### Registry credentials
# reg.wheat-dn42.net requires auth, so the kubelet needs a pull secret in the
# same namespace as the Deployment. REGISTRY_USER / REGISTRY_PASSWORD come from
# `sops decrypt secrets.yaml` in the devShell shellHook.
registry_user = os.getenv('REGISTRY_USER')
registry_password = os.getenv('REGISTRY_PASSWORD')
if not registry_user or not registry_password:
    fail('REGISTRY_USER and REGISTRY_PASSWORD must be set — enter the devShell (nix develop) so the sops shellHook can decrypt secrets.yaml')

k8s_yaml(secret_yaml_docker_registry(
    name='reg-wheat-dn42-creds',
    namespace='default',
    server='reg.wheat-dn42.net',
    username=registry_user,
    password=registry_password,
))

# Deploy app
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
        '--set', 'imagePullSecrets[0].name=reg-wheat-dn42-creds',
    ]
)
