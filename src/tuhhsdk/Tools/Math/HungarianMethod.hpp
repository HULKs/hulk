#pragma once

#include <Eigen/Dense>
#include <vector>

/**
 * @brief Hungarian Method for assignment problem(s)
 * "The Hungarian method is a combinatorial optimization algorithm that solves the assignment problem in polynomial time"
 *
 * Based on https://www.topcoder.com/community/data-science/data-science-tutorials/assignment-problem-and-hungarian-algorithm/
 */
class HungarianMethod
{
public:
  /**
   * @brief
   * @pre Input has to be nxn and n > 1
   * @param cost Cost matrix of the problem
   * @param minimize If the algorithm should find the minimum matching.
   * @return maximum matching
   */
  Eigen::Array2Xi findMaximumMatching(Eigen::MatrixXi& cost, bool minimize = false);

  EIGEN_MAKE_ALIGNED_OPERATOR_NEW
private:
  template<typename T>
  class SimpleQueue {
  public:
    void push(T& a)
    {
      data_.emplace_back(a);
    }

    T pop()
    {
      return data_[pRead++];
    }

    void clear()
    {
      data_.clear();
      pRead = 0;
    }

    void reserve(int count)
    {
      data_.reserve(count);
    }

    std::size_t size()
    {
      return data_.size() - pRead;
    }

    bool empty()
    {
      return size() == 0;
    }

  private:
    std::vector<T> data_;
    std::size_t pRead = 0;
  };

private:
  /// The cost matrix of the current problem
  Eigen::MatrixXi cost_;
  /// Number of sources
  int n_;
  /// Current number of matched sources
  int maxMatch_;
  /// Labels of x and labels of y
  Eigen::VectorXi xLabels_, yLabels_;
  /// Matching x to y and y to x
  Eigen::VectorXi xyMatching_, yxMatching_;
  /// Temporary sets searchedSources and searchedTargets for saving current used vertices
  Eigen::Matrix<bool, Eigen::Dynamic, 1> searchedSources_, searchedTargets_;
  /// Lowest cost from x element X to slack_[y]
  Eigen::VectorXi slack_;
  /// slackx_[y] is the index of vertex x from X such that slack_[y] == l(x) + l(y) - w(x,y)
  Eigen::VectorXi slackX_;
  /// Saves the current alternating path
  Eigen::VectorXi prev_;
  /// queue for bfs (breadth-first search)
  SimpleQueue<int> q_;

  /**
   * @brief Initialize the corresponding labels to new cost matrix
   */
  void initLabels();
  /**
   * @brief adds the given x to the tree.
   * @param x The vertex to add
   * @param prevX The vertex that is the previous vertex on the alternating path.
   */
  void addToTree(int x, int prevX);
  /**
   * @brief updates xLabels_ yLabels_ and slack.
   * Normally called if augmenting path was not found.
   */
  void updateLabels();
  /**
   * @brief Returns true iff an augmentedPath was found.
   * @param q The queue to use for BFS
   * @param exposedY The exposed vertex on the sink (Y) side. Will be set by this function.
   * @param lastX The vertex x that is farthest away from the root vertex.
   * @return True if an augmentable path was found.
   */
  bool breadthFirstAlternatingTreeSearch(SimpleQueue<int>& q, int& exposedY, int& lastX);
  /**
   *
   * @param q The queue to use for BFS
   * @param exposedY The exposed vertex on the sink (Y) side. Will be set by this function.
   * @param lastX The vertex x that is farthest away from the root vertex.
   * @return True if an augmentable path was found.
   */
  bool searchForExposedX(SimpleQueue<int>& q, int& exposedY, int& lastX);
  /**
   * @brief Calculates a maximum matching.
   */
  void augment();
};

