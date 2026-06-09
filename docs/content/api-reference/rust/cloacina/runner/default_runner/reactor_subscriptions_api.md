# cloacina::runner::default_runner::reactor_subscriptions_api <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Reactor-subscription API for the DefaultRunner (CLOACI-I-0100 / T-0600).

Surfaces a thin user-facing wrapper over the
`ReactorSubscriptionsDAL::subscribe / unsubscribe / list` operations.
The unfiltered registration path — "fire workflow on every reactor
firing for this tenant" — lives here. The optional Python filter
callback (`@trigger(reactor=...)`) is a follow-up surface.
