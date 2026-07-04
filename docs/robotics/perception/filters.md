## Ball Filter

`ball_filter.maximum_matching_cost` is the squared Mahalanobis threshold for
matching a ball hypothesis to a ball percept. The matching covariance combines
the hypothesis position covariance and the percept covariance in ground
coordinates.

The default value `5.9914646` is the 95% chi-square threshold for two degrees of
freedom. Higher values allow matches farther from the hypothesis, while lower
values reject percepts more aggressively.
