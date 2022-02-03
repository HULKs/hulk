let annotations = {};
let annotationCircles = [];

const loadAnnotations = async () => {
  const annotationsResponse = await fetch('/annotations.json');
  annotations = await annotationsResponse.json();
  const annotationIndicesResponse = await fetch('/annotation-indices.json');
  annotationCircles = (await annotationIndicesResponse.json()).map(annotationIndex => ({
    ...annotationIndex, // image, imageIndex, circleIndex, removed
    ...annotations[annotationIndex.image][annotationIndex.circleIndex], // centerX, centerY, radius
  }));
};
