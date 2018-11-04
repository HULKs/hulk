import './style/style.css';

interface Ofa {
  init: () => void;
}

const ofa: Ofa = require('./ofa');

ofa.init();
