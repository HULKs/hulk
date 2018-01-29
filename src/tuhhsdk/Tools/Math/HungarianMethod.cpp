#include "HungarianMethod.hpp"

#include <limits>
#include <algorithm>


Eigen::Array2Xi HungarianMethod::findMaximumMatching(Eigen::MatrixXi& cost, bool minimize)
{
  using Eigen::VectorXi;
  // Some preconditions
  assert(cost.rows() == cost.cols());
  assert(cost.cols() > 1);

  // Initialization
  Eigen::Array2Xi ret;
  cost_ = cost;
  maxMatch_ = 0;
  n_ = static_cast<int>(cost.cols());
  xyMatching_ = VectorXi::Constant(n_, -1);
  yxMatching_ = VectorXi::Constant(n_, -1);
  searchedSources_.resize(n_);
  searchedTargets_.resize(n_);
  q_.reserve(n_);
  q_.clear();

  // Hungarian method maximizes by default.
  if (minimize)
  {
    cost_ *= -1;
  }

  // Step 0
  initLabels();
  // Step 1 to 3
  augment();

  ret.resize(2, n_);
  for (int c = 0; c < ret.cols(); c++)
  {
    ret(0, c) = xyMatching_[c];
    ret(1, c) = yxMatching_[c];
  }

  return ret;
}

void HungarianMethod::initLabels()
{
  using Eigen::VectorXi;
  xLabels_ = VectorXi::Zero(n_);
  yLabels_ = VectorXi::Zero(n_);

  for (int x = 0; x < n_; x++)
  {
    xLabels_[x] = cost_.row(x).maxCoeff();
  }

  slack_ = VectorXi::Constant(n_, -1);
  slackX_ = VectorXi::Constant(n_, -1);
}

void HungarianMethod::addToTree(int x, int prevX)
{
  searchedSources_[x] = true;
  prev_[x] = prevX;

  assert(x != prevX);

  // update slacks, because the vertex x was added to S
  for (int y = 0; y < n_; y++)
  {
    if (xLabels_[x] + yLabels_[y] - cost_(x, y) < slack_[y])
    {
      slack_[y] = xLabels_[x] + yLabels_[y] - cost_(x, y);
      slackX_[y] = x;
    }
  }
}

void HungarianMethod::updateLabels()
{
  int delta = std::numeric_limits<int>::max();

  // calculate delta with slack_
  for (int y = 0; y < n_; y++)
  {
    if (!searchedTargets_[y])
    {
      delta = std::min(delta, slack_[y]);
    }
  }
  // update the x labels
  for (int x = 0; x < n_; x++)
  {
    if (searchedSources_[x])
    {
      xLabels_[x] -= delta;
    }
  }

  for (int y = 0; y < n_; y++)
  {
    // update y labels
    if (searchedTargets_[y])
    {
      yLabels_[y] += delta;
    }
    // update slack_ vector
    if (!searchedTargets_[y])
    {
      slack_[y] -= delta;
    }
  }
}

void HungarianMethod::augment()
{
  do
  {
    // storage of found exposedY and x to augment
    int exposedY = -1, lastX = -1;

    searchedSources_ = Eigen::Matrix<bool, Eigen::Dynamic, 1>::Constant(n_, false);
    searchedTargets_ = Eigen::Matrix<bool, Eigen::Dynamic, 1>::Constant(n_, false);

    // init set prev - for the alternating tree
    prev_ = Eigen::VectorXi::Constant(n_, -1);

    // Step 1: finding root of the tree
    q_.clear();
    for (int x = 0; x < n_; x++)
    {
      if (xyMatching_[x] == -1)
      {
        lastX = x;
        q_.push(x);
        // To find the root vertex while backtracking the tree
        prev_[x] = -2;
        // Include root in set S
        searchedSources_[x] = true;
        break;
      }
    }

    // initializing slack array (x is root)
    for (int y = 0; y < n_; y++)
    {
      slack_[y] = xLabels_[lastX] + yLabels_[y] - cost_(lastX, y);
      slackX_[y] = lastX;
    }


    // second part of augment() function
    bool augmentedPathFound = false;
    do
    {
      // This is more or less step 3 of the algorithm
      augmentedPathFound = breadthFirstAlternatingTreeSearch(q_, exposedY, lastX);
      if (augmentedPathFound)
      {
        break;
      }

      // Here follows step 2 of the algorithm
      // augmenting path not found, so improve labeling
      updateLabels();
      q_.clear();

      augmentedPathFound = searchForExposedX(q_, exposedY, lastX);
    } while (!augmentedPathFound);

    assert(exposedY >= 0);

    // increment matching
    maxMatch_++;
    // in this cycle we inverse edges along augmenting path
    for (int cx = lastX, cy = exposedY; cx != -2; cx = prev_[cx])
    {
      int ty = xyMatching_[cx];
      yxMatching_[cy] = cx;
      xyMatching_[cx] = cy;
      cy = ty;
    }
  } while (maxMatch_ != n_);
  // check whether matching is already perfect
}

bool HungarianMethod::breadthFirstAlternatingTreeSearch(HungarianMethod::SimpleQueue<int>& q, int& exposedY, int& lastX)
{
  // building tree with bfs cycle
  bool augmentedPathFound = false;
  while (!q.empty() && !augmentedPathFound)
  {
    // current vertex from X part
    int x = q.pop();
    // iterate through all edges in equality graph
    for (int y = 0; y < n_; y++)
    {
      if (cost_(x, y) == xLabels_[x] + yLabels_[y] && !searchedTargets_[y])
      {
        // exposed vertex in Y found - augmenting path exists!
        if (yxMatching_[y] == -1)
        {
          lastX = x;
          exposedY = y;
          augmentedPathFound = true;
          break;
        }

        // else just add y to T,
        searchedTargets_[y] = true;
        // add vertex yxMatching[y], which is matched with y, to the SimpleQueue
        q.push(yxMatching_[y]);
        // add edges (x,y) and (y,yxMatching[y]) to the tree
        addToTree(yxMatching_[y], x);
      }
    }
  }
  return augmentedPathFound;
}

bool HungarianMethod::searchForExposedX(HungarianMethod::SimpleQueue<int>& q, int& exposedY, int& lastX)
{
  bool augmentedPathFound = false;
  for (int y = 0; y < n_; y++)
  {
    // in this cycle we add edges that were added to the equality graph as a
    // result of improving the labeling, we add edge (slackX[y], y) to the
    // tree if and only if !Targets[y] && slack[y] == 0, also with this edge
    // we add another one (y, yxMatching[y]) or augment the matching, if y was exposed
    if (!searchedTargets_[y] && slack_[y] == 0)
    {
      // exposed vertex in Y found - augmenting path exists!
      if (yxMatching_[y] == -1)
      {
        lastX = slackX_[y];
        exposedY = y;
        augmentedPathFound = true;
        break;
      }
      else
      {
        // else just add y to targets,
        searchedTargets_[y] = true;
        if (!searchedSources_[yxMatching_[y]])
        {
          // add vertex yxMatching[y], which is matched with y, to the SimpleQueue
          q.push(yxMatching_[y]);
          // and add edges (x,y) and (y, yxMatching[y]) to the tree
          addToTree(yxMatching_[y], slackX_[y]);
        }
      }
    }
  }
  return augmentedPathFound;
}
