use derivative::Derivative;
use hickory_resolver::config::LookupIpStrategy::{Ipv4Only, Ipv6Only};
use hickory_resolver::config::{
    LookupIpStrategy, NameServerConfig, ResolveHosts, ResolverConfig, ResolverOpts,
};
use hickory_resolver::lookup_ip::LookupIp;
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::xfer::Protocol;
use hickory_resolver::{ResolveError, Resolver, TokioResolver};
use std::fmt::{Display, Formatter};
use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;

#[derive(Derivative, Debug)]
#[derivative(PartialEq, Eq, Hash)]
pub(crate) struct DnsServer {
    name: String,

    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    resolver4: Resolver<TokioConnectionProvider>,
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    resolver6: Resolver<TokioConnectionProvider>,
}

impl DnsServer {
    pub(crate) fn new_dns(ip: String) -> Result<Self, AddrParseError> {
        let addr: SocketAddr = SocketAddr::new(IpAddr::from_str(&ip)?, 53);

        let mut resolver_config = ResolverConfig::new();
        resolver_config.add_name_server(NameServerConfig::new(addr, Protocol::Udp));

        let mut builder4 = TokioResolver::builder_with_config(
            resolver_config.clone(),
            TokioConnectionProvider::default(),
        );
        apply_options(Ipv4Only, builder4.options_mut());

        let resolver4 = builder4.build();
        let mut builder6 = TokioResolver::builder_with_config(
            resolver_config.clone(),
            TokioConnectionProvider::default(),
        );
        apply_options(Ipv6Only, builder6.options_mut());

        let resolver6 = builder6.build();

        Ok(DnsServer {
            name: ip,
            resolver4,
            resolver6,
        })
    }
}

impl DnsServer {
    pub(crate) async fn new_dot(
        domain: String,
        system_resolver: &Resolver<TokioConnectionProvider>,
    ) -> Result<Self, ResolveError> {
        let lookup = system_resolver.lookup_ip(&domain).await?;

        let addrs: Vec<SocketAddr> = lookup
            .into_iter()
            .map(|ip| SocketAddr::new(ip, 853))
            .collect();

        assert!(!addrs.is_empty());

        let mut resolver_config = ResolverConfig::new();

        addrs.iter().for_each(|addr| {
            let mut name_server_config = NameServerConfig::new(*addr, Protocol::Tls);
            name_server_config.tls_dns_name = Some(domain.clone());
            resolver_config.add_name_server(name_server_config);
        });

        let mut builder4 = TokioResolver::builder_with_config(
            resolver_config.clone(),
            TokioConnectionProvider::default(),
        );
        apply_options(Ipv4Only, builder4.options_mut());

        let resolver4 = builder4.build();
        let mut builder6 = TokioResolver::builder_with_config(
            resolver_config.clone(),
            TokioConnectionProvider::default(),
        );
        apply_options(Ipv6Only, builder6.options_mut());

        let resolver6 = builder6.build();

        Ok(DnsServer {
            name: domain,
            resolver4,
            resolver6,
        })
    }

    pub(crate) async fn resolve4(&self, domain: &str) -> Result<LookupIp, ResolveError> {
        self.resolver4.lookup_ip(domain).await
    }

    pub(crate) async fn resolve6(&self, domain: &str) -> Result<LookupIp, ResolveError> {
        self.resolver6.lookup_ip(domain).await
    }
}

impl Display for DnsServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

fn apply_options(ip_strategy: LookupIpStrategy, options: &mut ResolverOpts) {
    options.timeout = Duration::from_secs(1);
    options.attempts = 1;
    options.cache_size = 0;
    options.try_tcp_on_error = false;
    options.use_hosts_file = ResolveHosts::Never;
    options.validate = false;

    options.ip_strategy = ip_strategy;
}
