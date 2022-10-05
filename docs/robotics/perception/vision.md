# Vision

TODO: Mention all modules or just the important ones?
Modules of questionable importance:

- Field color detection
- Camera matrix provider

TODO: Add images

The vision cycler runs twice in two separate threads to process the images from the top and bottom camera in parallel.

Image resolution is determined by the hardware interface, but is currently set to 640x480 pixels for performance reasons.
Most of the vision pipeline happens on a segmented version of the image for the same reason.

Each cycler instance waits for the hardware interface to deliver it's respective camera image and then begins executing the modules listed below.

## Camera Matrix Provider

## Field Color Detection

## Image Segmenter

The first major module in the vision pipeline is the image segmenter.
It iterates through the image and merges vertically adjacent pixels that are similar.
This reduces the amount of elements subsequent have to process.
Instead of 480 pixels, each vertical scan line is reduced to just a dozen or so segments, depending on the image.
A stride can be set to only generate scanlines for every n-th pixel column.
Furthermore, segments which are above the horizon or overlap the robots limbs are discarded, resulting in a sparse image.

Each segment contains it's location, color, edge types, and a pre-calculated classification of field color intensity.

## Field Border Detection

Estimates the location of the upper field border in the image.

## Segment Filter

The image segments are further reduced by removing all segments that are considered field color.
Segments above the field border are also removed.

## Line Detection

Using the filtered segments, field lines are detected by looking for white segments of appropriate length.
For each segment the gradient at each end is calculated using the [Sobel operator](https://en.wikipedia.org/wiki/Sobel_operator).
The segment is discarded if the gradients do not sufficiently point in opposing directions, i.e. the ends do not lie on opposite edges of a line.

The center of each remaining segment is then used in a [RANSAC](https://en.wikipedia.org/wiki/Ransac) line fitting algorithm.
Found lines are projected onto the ground and then checked against those found previously to see if they are either parallel or orthogonal to each other.

## Perspective Grid Candidate Provider

This module generates candidates for the ball detection.
Starting the from the bottom of the image, rows of circles are generated where the circle radius matches the projected ball radius in the row's center.
Candidates are only generated when there exists at least one filtered segment whose center is inside the candidate circle's bounding box.

## Ball Detection

For each perspective grid candidate a series of artifical neural networks is used to determine whether it contains a ball as well as the balls location and radius.
First, a sample is extraced from the original, unsegmented image, centered around the candidate but with a larger radius.
The sample is scaled up or down to 32x32 pixels, regardless of the size in the image.

The first neural network to run on the image is called the "preclassifier", which is a small but cheap model to quickly filter out candidates that are clearly not a ball.

If the preclassifier claims to have found a ball a larger, more accurate network, the "classifier" is executed to finally determine whether the candidate contains a ball or not.

Once the main classifier finds a ball, a third neural network, the "positioner" is used to determine the location and size of the ball within the sample.
These values are then transformed back into the coordinate frame of the image and then projected onto the field to determine the final location of the detected ball.

![Ball Detection Debug View](./ball_candidates.jpg)

TODO: Implement this view in twix and update screenshot

Debug view showing

 - blue circle: candidates from the perspective grid
 - green circle: positioner network output
 - white circle: clustered ball location
 - red circle: current ball model, see [filters](./filters.md)
 - black text: preclassifier confidence

## Robot Detection

Warning: This module is still work in progress.

For detecting robots, a clustering algorithm runs through each vertical scanline of the filtered image segments, ignoring segments that have been previously used by the ball detection or line detection.
The last (bottom most) cluster in each scanline is then projected to the ground and clustered first using the score-weighted distance and then again using cones.
