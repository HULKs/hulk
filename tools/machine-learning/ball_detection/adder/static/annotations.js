let existingAnnotations = {};
let addedAnnotations = {};
let annotationImages = [];
let annotationImageSizes = [];

const loadAnnotations = async () => {
  const existingAnnotationsResponse = await fetch('/existing-annotations.json');
  existingAnnotations = await existingAnnotationsResponse.json();
  const addedAnnotationsResponse = await fetch('/added-annotations.json');
  addedAnnotations = await addedAnnotationsResponse.json();
  annotationImages = Object.keys(addedAnnotations);
  annotationImages.sort();
  annotationImageSizes = annotationImages.map(() => ({
    width: 0,
    height: 0,
  }));
};
