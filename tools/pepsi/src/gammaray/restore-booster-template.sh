# Discard template overrides from older gammaray deployments. The underlying
# installation supplies the matching vendor template without a host-side copy.
if [[ "$root" != / ]]; then
    config=/opt/booster/config/containerd/config.yaml
    if ! cmp -s "$root$config" "$config"; then
        updated=$(mktemp "${config}.XXXXXX")
        cp --preserve=mode,ownership "$root$config" "$updated"
        mv "$updated" "$config"
    fi
fi
