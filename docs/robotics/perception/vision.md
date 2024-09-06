# Vision

TODO: Mention all nodes or just the important ones?
Nodes of questionable importance:

-   Field color detection
-   Camera matrix provider

TODO: Add images

The vision cycler runs twice in two separate threads to process the images from the top and bottom camera in parallel.

Image resolution is determined by the hardware interface, but is currently set to 640x480 pixels for performance reasons.
Most of the vision pipeline happens on a segmented version of the image for the same reason.

Each cycler instance waits for the hardware interface to deliver it's respective camera image and then begins executing the nodes listed below.

## Camera Matrix Provider

## Field Color Detection

## Image Segmenter

The first major node in the vision pipeline is the image segmenter.
It iterates through the image and merges vertically adjacent pixels that are similar.
This reduces the amount of elements subsequent nodes have to process.
Instead of 480 pixels, each vertical scan line is reduced to just a dozen or so segments, depending on the image.
A stride can be set to only generate scanlines for every n-th pixel column.
Furthermore, segments which are above the horizon or overlap the robots limbs are discarded, resulting in a sparse image.

Each segment contains it's location, color, edge types, and a pre-calculated classification of field color intensity.

## Field Border Detection

Estimates the location of the upper field border in the image by finding the first pixels from the top that are roughly field-colored and fitting a line through them.

## Segment Filter

The image segments are further reduced by removing all segments that are considered field color to only preserve relevant features.
Segments above the field border are also removed.

## Line Detection

Using the filtered segments, field lines are detected by looking for white segments of appropriate length.
For each segment the brightness gradient at each end is calculated using the [Sobel operator](https://en.wikipedia.org/wiki/Sobel_operator).
The segment is discarded if the gradients are not sufficiently steep upwards and downwards, i.e. the segment borders do not lie on opposite flanks of a field line.

The center of each remaining segment is then used in a [RANSAC](https://en.wikipedia.org/wiki/Ransac) line fitting algorithm.
Found lines are projected onto the ground and then checked against those found previously to see if they are either parallel or orthogonal to each other.

TODO: Why check parallelism and orthogonality? What do we do with this information?

## Perspective Grid Candidate Provider

This node generates candidates for the [Ball Detection](#ball-detection).
Starting from the bottom of the image, rows of circles are generated where the circle size matches the projected ball in the row's center.
Candidates are only generated when the center of at least one filtered segment is inside the candidate circle's bounding box.

## Ball Detection

For each perspective grid candidate a series of artifical neural networks is used to determine whether it contains a ball as well as the balls location and radius.
First, a slightly larger sample centered around the candidate is extracted from the raw image.
This sample is scaled up or down to 32x32 pixels, regardless of the size in the raw image.

The first neural network to run on the image is called the "preclassifier", which is a small but cheap model to quickly filter out candidates that are clearly not a ball.

If the preclassifier claims to have found a ball, a larger and more accurate network, the "classifier", is executed to decide whether the candidate contains a ball or not.

Once the classifier finds a ball, a third neural network, the "positioner", is used to determine the location and size of the ball within the sample.
These values are then transformed back into the coordinate frame of the image and then projected onto the field to determine the final location of the detected ball.

!!! todo

    Clustering

### Image Debug View

![Ball Detection Debug View](./ball_candidates.jpg)
Image taken from the game HULKs vs. SPQR, 2024-07-20 at RoboCup 2024 in Eindhoven.

-   blue circles and boxes: candidates from the perspective grid
-   green circle: positioner network output
-   red circle: current ball model, see [filters](./filters.md)

### Ball Candidates Debug View

![Raw Ball Candidates](./raw_ball_candidates.jpg)

Ball Candidates debug view showing the 32x32 pixel samples used by the preclassifier.

## Feet Detection

For detecting robots, a clustering algorithm iterates over each vertical scanline of the filtered image segments, ignoring segments that have been previously used by the ball detection or line detection.
The last (bottom most) cluster border in each scanline is then projected to the ground and clustered using the distance between each new cluster point and the cluster mean.
These clusters are then filtered again for a minimum cluster width and a minimum number of points in the cluster.

A major disadvantage of this approach is that it relies heavily on the number of segments and their size, which is influenced by lighting conditions and vision tuning.
On the other hand, it is very lightweight and can be quite effective if properly tuned.

![Feet Detection Image View](./feet_detection_image_view.jpg)
![Feet Detection Map View](./feet_detection_map_view.jpg)
Image taken from the game HULKs vs. SPQR, 2024-07-20 at RoboCup 2024 in Eindhoven.
