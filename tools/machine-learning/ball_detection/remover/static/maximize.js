let maximizedCellElement = null;

let minimizedBoxX = null;
let minimizedBoxY = null;
let minimizedBoxWidth = null;
let minimizedBoxHeight = null;
let maximizeAnimationBox = null;

let minimizedImageX = null;
let minimizedImageY = null;
let minimizedImageWidth = null;
let minimizedImageHeight = null;
let maximizeAnimationImage = null;

const map = (value, sourceStart, sourceEnd, targetStart, targetEnd) => {
  return ((value - sourceStart) / (sourceEnd - sourceStart) * (targetEnd - targetStart)) + targetStart;
};

const maximize = cellElement => {
  maximizedCellElement = cellElement;
  const i = parseInt(maximizedCellElement.id);
  const circle = annotationCircles[i];
  addTelemetryMessage({
    'timestamp': (new Date).toISOString(),
    'type': 'maximize',
    'cellId': i,
    'image': circle.image,
  });
  const svg = maximizedCellElement.querySelector('div>svg');
  const boundingBox = svg.getBoundingClientRect();
  minimizedBoxX = window.scrollX + boundingBox.left;
  minimizedBoxY = window.scrollY + boundingBox.top;
  minimizedBoxWidth = boundingBox.width;
  minimizedBoxHeight = boundingBox.height;
  const maximizedBoxX = window.scrollX;
  const maximizedBoxY = window.scrollY;
  const maximizedBoxWidth = window.innerWidth;
  const maximizedBoxHeight = window.innerHeight;

  const boxElement = document.querySelector('#box');
  const image = new Image;
  image.onload = () => {
    boxElement.setAttribute('viewBox', `0 0 ${image.naturalWidth} ${image.naturalHeight}`);
    const imageElement = boxElement.querySelector('image');
    imageElement.setAttributeNS(null, 'href', image.currentSrc);

    const cropStartX = circle.centerX - (circle.radius * config.cropScaleFactor);
    const cropStartY = circle.centerY - (circle.radius * config.cropScaleFactor);
    const cropEndX = circle.centerX + (circle.radius * config.cropScaleFactor);
    const cropEndY = circle.centerY + (circle.radius * config.cropScaleFactor);
    const viewBoxOffsetX = image.naturalWidth < image.naturalHeight ? (image.naturalHeight - image.naturalWidth) / 2 : 0;
    const viewBoxOffsetY = image.naturalWidth < image.naturalHeight ? 0 : (image.naturalWidth - image.naturalHeight) / 2;
    minimizedImageX = map(0, cropStartX, cropEndX, -viewBoxOffsetX, image.naturalWidth + viewBoxOffsetX);
    minimizedImageY = map(0, cropStartY, cropEndY, -viewBoxOffsetY, image.naturalHeight + viewBoxOffsetY);
    const minimizedImageEndX = map(image.naturalWidth, cropStartX, cropEndX, -viewBoxOffsetX, image.naturalWidth + viewBoxOffsetX);
    minimizedImageWidth = minimizedImageEndX - minimizedImageX;
    const minimizedImageEndY = map(image.naturalHeight, cropStartY, cropEndY, -viewBoxOffsetY, image.naturalHeight + viewBoxOffsetY);
    minimizedImageHeight = minimizedImageEndY - minimizedImageY;

    const timing = {
      duration: 500,
      easing: 'cubic-bezier(0.4, 0.0, 0.2, 1)',
      fill: 'forwards',
    };
    if (maximizeAnimationBox !== null) {
      maximizeAnimationBox.pause();
    }
    maximizeAnimationBox = boxElement.animate([
      {
        left: `${minimizedBoxX}px`,
        top: `${minimizedBoxY}px`,
        width: `${minimizedBoxWidth}px`,
        height: `${minimizedBoxHeight}px`,
      },
      {
        left: `${maximizedBoxX}px`,
        top: `${maximizedBoxY}px`,
        width: `${maximizedBoxWidth}px`,
        height: `${maximizedBoxHeight}px`,
      },
    ], timing);
    maximizeAnimationBox.onfinish = () => {
      maximizeAnimationBox = null;
    };

    if (maximizeAnimationImage !== null) {
      maximizeAnimationImage.pause();
    }
    maximizeAnimationImage = imageElement.animate([
      {
        x: `${minimizedImageX}px`,
        y: `${minimizedImageY}px`,
        width: `${minimizedImageWidth}px`,
        height: `${minimizedImageHeight}px`,
      },
      {
        x: '0px',
        y: '0px',
        width: `${image.naturalWidth}px`,
        height: `${image.naturalHeight}px`,
      }
    ], timing);
    maximizeAnimationImage.onfinish = () => {
      maximizeAnimationImage = null;
    };

    boxElement.style.visibility = 'visible';
  };
  image.src = `/image/${annotationCircles[i].imageIndex}`;
};

const minimize = () => {
  const boxElement = document.querySelector('#box');
  const currentStyleBox = window.getComputedStyle(boxElement);
  const imageElement = boxElement.querySelector('image');
  const currentStyleImage = window.getComputedStyle(imageElement);

  const i = parseInt(maximizedCellElement.id);
  const circle = annotationCircles[i];
  addTelemetryMessage({
    'timestamp': (new Date).toISOString(),
    'type': 'minimize',
    'cellId': i,
    'image': circle.image,
  });

  const timing = {
    duration: 400,
    easing: 'cubic-bezier(0.4, 0.0, 0.2, 1)',
    fill: 'forwards',
  };
  if (maximizeAnimationBox !== null) {
    maximizeAnimationBox.pause();
  }
  maximizeAnimationBox = boxElement.animate([
    {
      left: currentStyleBox.left,
      top: currentStyleBox.top,
      width: currentStyleBox.width,
      height: currentStyleBox.height,
    },
    {
      left: `${minimizedBoxX}px`,
      top: `${minimizedBoxY}px`,
      width: `${minimizedBoxWidth}px`,
      height: `${minimizedBoxHeight}px`,
    },
  ], timing);
  maximizeAnimationBox.onfinish = () => {
    maximizeAnimationBox = null;
    boxElement.style.visibility = 'hidden';
  };

  if (maximizeAnimationImage !== null) {
    maximizeAnimationImage.pause();
  }
  maximizeAnimationImage = imageElement.animate([
    {
      x: currentStyleImage.x,
      y: currentStyleImage.y,
      width: currentStyleImage.width,
      height: currentStyleImage.height,
    },
    {
      x: `${minimizedImageX}px`,
      y: `${minimizedImageY}px`,
      width: `${minimizedImageWidth}px`,
      height: `${minimizedImageHeight}px`,
    },
  ], timing);
  maximizeAnimationImage.onfinish = () => {
    maximizeAnimationImage = null;
  };
};
