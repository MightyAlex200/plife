Writing Configuration Files
===========================

Configuration files are written in [YAML][yaml]. A configuration consists of a **ruleset**, **walls**, and **points**:

```yaml
---
ruleset:
    types: 20
    attractions:
        # etc. etc.
walls:
    type: none
points: 5000
```
(All YAML files should start with three dashes `---`)

Distributions
-------------
Anywhere a **distribution** is expected, you may enter one of:
- a number (`57`, `2.85`, `-0.03`, etc.)
- a [uniform distribution][uniform]:
    ```yaml
    min: -50
    max: 100
    ```
- a [normal distribution][normal]:
    ```yaml
    mean: 0.0
    std: 80.0
    ```

Rulesets
--------
Ruleset definitions may be either **procedural** or **precise**. See the [plife wiki][plife-wiki] for more information on what these values mean.

### Procedural Ruleset
Using a procedural ruleset will generate a different ruleset every run based on the values in it.
```yaml
# ...
ruleset:
    types: (distribution)
    attractions: (distribution)
    min_r: (distribution)
    max_r: (distribution)
    attractions: (distribution)
# ...
```
### Precise Ruleset
A precise ruleset will not change between runs, but grows in effort at n² because each particle type must know how to interact with every other particle type.
```yaml
# ...
ruleset:
    types:
        - attractions: [0.0, 1.0] # example values, NOT distributions
          min_r: [2.0, 0.02]
          max_r: [5.0, 10.0]
        - attractions: [5.0, -0.1]
          min_r: [0.01, 0.2]
          max_r: [0.8, 5.0]
    friction: (distribution)
# ...
```

Walls
-----
Walls are comparatively simple! There are three types:
- `none`
- `wrapping`
- `square`
If the type is `wrapping` or `square`, then a `dist` property is required.
```yaml
# ...
walls:
    type: wrapping
    dist: (distribution)
# ...
```

Points
------
The definition of `points` may be simply a distribution:
```yaml
# ...
points:
    min: 500
    max: 1000
# ...
```
or they may be a list of spawn points:
```yaml
# ...
points:
    - num: (distribution)
      x: (distribution)
      y: (distribution)
    - num: (distribution)
      # ...
# ...
```

[yaml]: https://yaml.org
[uniform]: https://mathworld.wolfram.com/UniformDistribution.html
[normal]: https://mathworld.wolfram.com/NormalDistribution.html
[plife-wiki]: https://plife.fandom.com/wiki/Plife