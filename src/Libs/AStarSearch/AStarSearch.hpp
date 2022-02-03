// Taken from https://github.com/justinhj/astar-algorithm-cpp/
/*
A* Algorithm Implementation using STL is
Copyright (C)2001-2005 Justin Heyes-Jones
Permission is given by the author to freely redistribute and
include this code in any program as long as this credit is
given where due.

  COVERED CODE IS PROVIDED UNDER THIS LICENSE ON AN "AS IS" BASIS,
  WITHOUT WARRANTY OF ANY KIND, EITHER EXPRESSED OR IMPLIED,
  INCLUDING, WITHOUT LIMITATION, WARRANTIES THAT THE COVERED CODE
  IS FREE OF DEFECTS, MERCHANTABLE, FIT FOR A PARTICULAR PURPOSE
  OR NON-INFRINGING. THE ENTIRE RISK AS TO THE QUALITY AND
  PERFORMANCE OF THE COVERED CODE IS WITH YOU. SHOULD ANY COVERED
  CODE PROVE DEFECTIVE IN ANY RESPECT, YOU (NOT THE INITIAL
  DEVELOPER OR ANY OTHER CONTRIBUTOR) ASSUME THE COST OF ANY
  NECESSARY SERVICING, REPAIR OR CORRECTION. THIS DISCLAIMER OF
  WARRANTY CONSTITUTES AN ESSENTIAL PART OF THIS LICENSE. NO USE
  OF ANY COVERED CODE IS AUTHORIZED HEREUNDER EXCEPT UNDER
  THIS DISCLAIMER.

  Use at your own risk!
*/

#ifndef STLASTAR_H
#define STLASTAR_H
// used for text debugging
#include <iostream>
#include <stdio.h>
//#include <conio.h>
#include <assert.h>

// stl includes
#include <algorithm>
#include <cfloat>
#include <memory>
#include <set>
#include <vector>

// fast fixed size memory allocator, used for fast node memory management
#include "Libs/AStarSearch/fsa.h"

// Fixed size memory allocator can be disabled to compare performance
// Uses std new and delete instead if you turn it off
#define USE_FSA_MEMORY 0

// The AStar search class. UserNode is the users Node type
template <class UserNode>
class AStarSearch
{

public: // data
  enum
  {
    SEARCH_STATE_NOT_INITIALISED,
    SEARCH_STATE_SEARCHING,
    SEARCH_STATE_SUCCEEDED,
    SEARCH_STATE_FAILED,
    SEARCH_STATE_OUT_OF_MEMORY,
    SEARCH_STATE_INVALID
  };


  // A node represents a possible state in the search
  // The user provided state type is included inside this type

public:
  class Node
  {
  public:
    Node* parent; // used during the search to record the parent of successor nodes
    Node* child;  // used after the search for the application to view the search in reverse

    float g; // cost of this node + it's predecessors
    float h; // heuristic estimate of distance to goal
    float f; // sum of cumulative cost of predecessors and self and heuristic

    Node()
      : parent(0)
      , child(0)
      , g(0.0f)
      , h(0.0f)
      , f(0.0f)
    {
    }

    std::shared_ptr<UserNode> userNode;
  };


  // For sorting the heap the STL needs compare function that lets us compare
  // the f value of two nodes

  class HeapCompare
  {
  public:
    bool operator()(const Node* x, const Node* y) const
    {
      return x->f > y->f;
    }
  };


public: // methods
  // constructor just initialises private data
  AStarSearch()
    : state_(SEARCH_STATE_NOT_INITIALISED)
    , currentSolutionNode_(NULL)
    ,
#if USE_FSA_MEMORY
    fixedSizeAllocator_(1000)
    ,
#endif
    allocateNodeCount_(0)
    , cancelRequest_(false)
  {
  }

  //   AStarSearch(int MaxNodes)
  //     : state_(SEARCH_STATE_NOT_INITIALISED)
  //     , currentSolutionNode_(NULL)
  //     ,
  // #if USE_FSA_MEMORY
  //     fixedSizeAllocator_(MaxNodes)
  //     ,
  // #endif
  //     allocateNodeCount_(0)
  //     , cancelRequest_(false)
  //   {
  //   }

  // call at any time to cancel the search and free up all the memory
  void cancelSearch()
  {
    cancelRequest_ = true;
  }

  // Set Start and goal states
  void setStartAndGoalNodes(std::shared_ptr<UserNode> startNode, std::shared_ptr<UserNode> goalNode)
  {
    cancelRequest_ = false;

    start_ = allocateNode();
    goal_ = allocateNode();

    assert((start_ != NULL && goal_ != NULL));

    start_->userNode = startNode;
    goal_->userNode = goalNode;

    state_ = SEARCH_STATE_SEARCHING;

    // Initialise the AStar specific parts of the Start Node
    // The user only needs fill out the state information

    start_->g = 0;
    start_->h = start_->userNode->goalDistanceEstimate(goal_->userNode);
    start_->f = start_->g + start_->h;
    start_->parent = 0;

    // Push the start node on the Open list
    openList_.push_back(start_); // heap now unsorted

    // Sort back element into heap
    push_heap(openList_.begin(), openList_.end(), HeapCompare());

    // Initialise counter for search steps
    stepCount_ = 0;
  }

  // Advances search one step
  unsigned int searchStep()
  {
    // Firstly break if the user has not initialised the search
    assert((state_ > SEARCH_STATE_NOT_INITIALISED) && (state_ < SEARCH_STATE_INVALID));

    // Next I want it to be safe to do a searchstep once the search has succeeded...
    if ((state_ == SEARCH_STATE_SUCCEEDED) || (state_ == SEARCH_STATE_FAILED))
    {
      return state_;
    }

    // Failure is defined as emptying the open list as there is nothing left to
    // search...
    // New: Allow user abort
    if (openList_.empty() || cancelRequest_)
    {
      freeAllNodes();
      state_ = SEARCH_STATE_FAILED;
      return state_;
    }

    // Incremement step count
    stepCount_++;

    // Pop the best node (the one with the lowest f)
    Node* node = openList_.front(); // get pointer to the node
    pop_heap(openList_.begin(), openList_.end(), HeapCompare());
    openList_.pop_back();

    // Check for the goal, once we pop that we're done
    if (node->userNode->isGoal(goal_->userNode))
    {
      // The user is going to use the goal node he passed in
      // so copy the parent pointer of n
      goal_->parent = node->parent;
      goal_->g = node->g;

      // A special case is that the goal was passed in as the start state
      // so handle that here
      if (false == node->userNode->isSameNode(start_->userNode))
      {
        freeNode(node);

        // set the child pointers in each node (except goal node which has no child)
        Node* nodeChild = goal_;
        Node* nodeParent = goal_->parent;

        do
        {
          nodeParent->child = nodeChild;

          nodeChild = nodeParent;
          nodeParent = nodeParent->parent;

        } while (nodeChild != start_); // Start is always the first node by definition
      }

      // delete nodes that aren't needed for the solution
      freeUnusedNodes();

      state_ = SEARCH_STATE_SUCCEEDED;

      return state_;
    }
    else // not goal
    {

      // We now need to generate the successors of this node
      // The user helps us to do this, and we keep the new nodes in
      // successors ...
      successors_.clear(); // empty vector of successors nodes to n

      // User provides this functions and uses AddSuccessor to add each successor of
      // node 'n' to successor
      bool ret = node->userNode->getSuccessors(this, node->parent ? node->parent->userNode : NULL,
                                               goal_->userNode);

      if (!ret)
      {

        typename std::vector<Node*>::iterator successor;

        // free the nodes that may previously have been added
        for (successor = successors_.begin(); successor != successors_.end(); successor++)
        {
          freeNode((*successor));
        }

        successors_.clear(); // empty vector of successor nodes to n

        // free up everything else we allocated
        freeNode((node));
        freeAllNodes();

        state_ = SEARCH_STATE_OUT_OF_MEMORY;
        return state_;
      }

      // Now handle each successor to the current node ...
      for (typename std::vector<Node*>::iterator successor = successors_.begin();
           successor != successors_.end(); successor++)
      {

        // 	The g value for this successor ...
        float newg = node->g + node->userNode->getCost((*successor)->userNode);

        // Now we need to find whether the node is on the open or closed lists
        // If it is but the node that is already on them is better (lower g)
        // then we can forget about this successor

        // First linear search of open list to find node

        typename std::vector<Node*>::iterator openListResult;

        for (openListResult = openList_.begin(); openListResult != openList_.end();
             openListResult++)
        {
          if ((*openListResult)->userNode->isSameNode((*successor)->userNode))
          {
            break;
          }
        }

        if (openListResult != openList_.end())
        {

          // we found this state on open

          if ((*openListResult)->g <= newg)
          {
            freeNode((*successor));

            // the one on Open is cheaper than this one
            continue;
          }
        }

        typename std::vector<Node*>::iterator closedListResult;

        for (closedListResult = closedList_.begin(); closedListResult != closedList_.end();
             closedListResult++)
        {
          if ((*closedListResult)->userNode->isSameNode((*successor)->userNode))
          {
            break;
          }
        }

        if (closedListResult != closedList_.end())
        {

          // we found this state on closed

          if ((*closedListResult)->g <= newg)
          {
            // the one on Closed is cheaper than this one
            freeNode((*successor));

            continue;
          }
        }

        // This node is the best node so far with this particular state
        // so lets keep it and set up its AStar specific data ...

        (*successor)->parent = node;
        (*successor)->g = newg;
        (*successor)->h = (*successor)->userNode->goalDistanceEstimate(goal_->userNode);
        (*successor)->f = (*successor)->g + (*successor)->h;

        // Successor in closed list
        // 1 - Update old version of this node in closed list
        // 2 - Move it from closed to open list
        // 3 - Sort heap again in open list

        if (closedListResult != closedList_.end())
        {
          // Update closed node with successor node AStar data
          //*(*closedlist_result) = *(*successor);
          (*closedListResult)->parent = (*successor)->parent;
          (*closedListResult)->g = (*successor)->g;
          (*closedListResult)->h = (*successor)->h;
          (*closedListResult)->f = (*successor)->f;

          // Free successor node
          freeNode((*successor));

          // Push closed node into open list
          openList_.push_back((*closedListResult));

          // Remove closed node from closed list
          closedList_.erase(closedListResult);

          // Sort back element into heap
          push_heap(openList_.begin(), openList_.end(), HeapCompare());

          // Fix thanks to ...
          // Greg Douglas <gregdouglasmail@gmail.com>
          // who noticed that this code path was incorrect
          // Here we have found a new state which is already CLOSED
        }

        // Successor in open list
        // 1 - Update old version of this node in open list
        // 2 - sort heap again in open list

        else if (openListResult != openList_.end())
        {
          // Update open node with successor node AStar data
          //*(*openlist_result) = *(*successor);
          (*openListResult)->parent = (*successor)->parent;
          (*openListResult)->g = (*successor)->g;
          (*openListResult)->h = (*successor)->h;
          (*openListResult)->f = (*successor)->f;

          // Free successor node
          freeNode((*successor));

          // re-make the heap
          // make_heap rather than sort_heap is an essential bug fix
          // thanks to Mike Ryynanen for pointing this out and then explaining
          // it in detail. sort_heap called on an invalid heap does not work
          make_heap(openList_.begin(), openList_.end(), HeapCompare());
        }

        // New successor
        // 1 - Move it from successors to open list
        // 2 - sort heap again in open list

        else
        {
          // Push successor node into open list
          openList_.push_back((*successor));

          // Sort back element into heap
          push_heap(openList_.begin(), openList_.end(), HeapCompare());
        }
      }

      // push n onto Closed, as we have expanded it now

      closedList_.push_back(node);

    } // end else (not goal so expand)

    return state_; // Succeeded bool is false at this point.
  }

  // User calls this to add a successor to a list of successors
  // when expanding the search frontier
  bool addSuccessor(std::shared_ptr<UserNode> userNode)
  {
    Node* node = allocateNode();

    if (node)
    {
      node->userNode = userNode;

      successors_.push_back(node);

      return true;
    }

    return false;
  }

  // Free the solution nodes
  // This is done to clean up all used Node memory when you are done with the
  // search
  void freeSolutionNodes()
  {
    Node* node = start_;

    if (start_->child)
    {
      do
      {
        Node* del = node;
        node = node->child;
        freeNode(del);

        del = NULL;

      } while (node != goal_);

      freeNode(node); // Delete the goal
    }
    else
    {
      // if the start node is the solution we need to just delete the start and goal
      // nodes
      freeNode(start_);
      freeNode(goal_);
    }
  }

  // Functions for traversing the solution

  // Get start node
  std::shared_ptr<UserNode> getSolutionStart()
  {
    currentSolutionNode_ = start_;
    if (start_)
    {
      return start_->userNode;
    }
    else
    {
      return NULL;
    }
  }

  // Get next node
  std::shared_ptr<UserNode> getSolutionNext()
  {
    if (currentSolutionNode_)
    {
      if (currentSolutionNode_->child)
      {

        Node* child = currentSolutionNode_->child;

        currentSolutionNode_ = currentSolutionNode_->child;

        return child->userNode;
      }
    }

    return NULL;
  }

  // Get end node
  std::shared_ptr<UserNode> getSolutionEnd()
  {
    currentSolutionNode_ = goal_;
    if (goal_)
    {
      return goal_->userNode;
    }
    else
    {
      return NULL;
    }
  }

  // Step solution iterator backwards
  std::shared_ptr<UserNode> getSolutionPrev()
  {
    if (currentSolutionNode_)
    {
      if (currentSolutionNode_->parent)
      {

        Node* parent = currentSolutionNode_->parent;

        currentSolutionNode_ = currentSolutionNode_->parent;

        return parent->userNode;
      }
    }

    return NULL;
  }

  // Get final cost of solution
  // Returns FLT_MAX if goal is not defined or there is no solution
  float getSolutionCost()
  {
    if (goal_ && state_ == SEARCH_STATE_SUCCEEDED)
    {
      return goal_->g;
    }
    else
    {
      return FLT_MAX;
    }
  }

  // For educational use and debugging it is useful to be able to view
  // the open and closed list at each step, here are two functions to allow that.

  std::shared_ptr<UserNode> getOpenListStart()
  {
    float f, g, h;
    return getOpenListStart(f, g, h);
  }

  std::shared_ptr<UserNode> getOpenListStart(float& f, float& g, float& h)
  {
    iterDbgOpen = openList_.begin();
    if (iterDbgOpen != openList_.end())
    {
      f = (*iterDbgOpen)->f;
      g = (*iterDbgOpen)->g;
      h = (*iterDbgOpen)->h;
      return (*iterDbgOpen)->userNode;
    }

    return NULL;
  }

  std::shared_ptr<UserNode> getOpenListNext()
  {
    float f, g, h;
    return getOpenListNext(f, g, h);
  }

  std::shared_ptr<UserNode> getOpenListNext(float& f, float& g, float& h)
  {
    iterDbgOpen++;
    if (iterDbgOpen != openList_.end())
    {
      f = (*iterDbgOpen)->f;
      g = (*iterDbgOpen)->g;
      h = (*iterDbgOpen)->h;
      return (*iterDbgOpen)->userNode;
    }

    return NULL;
  }

  std::shared_ptr<UserNode> getClosedListStart()
  {
    float f, g, h;
    return getClosedListStart(f, g, h);
  }

  std::shared_ptr<UserNode> getClosedListStart(float& f, float& g, float& h)
  {
    iterDbgClosed = closedList_.begin();
    if (iterDbgClosed != closedList_.end())
    {
      f = (*iterDbgClosed)->f;
      g = (*iterDbgClosed)->g;
      h = (*iterDbgClosed)->h;

      return (*iterDbgClosed)->userNode;
    }

    return NULL;
  }

  std::shared_ptr<UserNode> getClosedListNext()
  {
    float f, g, h;
    return getClosedListNext(f, g, h);
  }

  std::shared_ptr<UserNode> getClosedListNext(float& f, float& g, float& h)
  {
    iterDbgClosed++;
    if (iterDbgClosed != closedList_.end())
    {
      f = (*iterDbgClosed)->f;
      g = (*iterDbgClosed)->g;
      h = (*iterDbgClosed)->h;

      return (*iterDbgClosed)->userNode;
    }

    return NULL;
  }

  // Get the number of steps

  int getStepCount()
  {
    return stepCount_;
  }

  void ensureMemoryFreed()
  {
#if USE_FSA_MEMORY
    assert(allocateNodeCount_ == 0);
#endif
  }

private: // methods
  // This is called when a search fails or is cancelled to free all used
  // memory
  void freeAllNodes()
  {
    // iterate open list and delete all nodes
    typename std::vector<Node*>::iterator iterOpen = openList_.begin();

    while (iterOpen != openList_.end())
    {
      Node* node = (*iterOpen);
      freeNode(node);

      iterOpen++;
    }

    openList_.clear();

    // iterate closed list and delete unused nodes
    typename std::vector<Node*>::iterator iterClosed;

    for (iterClosed = closedList_.begin(); iterClosed != closedList_.end(); iterClosed++)
    {
      Node* node = (*iterClosed);
      freeNode(node);
    }

    closedList_.clear();

    // delete the goal

    freeNode(goal_);
  }


  // This call is made by the search class when the search ends. A lot of nodes may be
  // created that are still present when the search ends. They will be deleted by this
  // routine once the search ends
  void freeUnusedNodes()
  {
    // iterate open list and delete unused nodes
    typename std::vector<Node*>::iterator iterOpen = openList_.begin();

    while (iterOpen != openList_.end())
    {
      Node* node = (*iterOpen);

      if (!node->child)
      {
        freeNode(node);

        node = NULL;
      }

      iterOpen++;
    }

    openList_.clear();

    // iterate closed list and delete unused nodes
    typename std::vector<Node*>::iterator iterClosed;

    for (iterClosed = closedList_.begin(); iterClosed != closedList_.end(); iterClosed++)
    {
      Node* node = (*iterClosed);

      if (!node->child)
      {
        freeNode(node);
        node = NULL;
      }
    }

    closedList_.clear();
  }

  // Node memory management
  Node* allocateNode()
  {

#if !USE_FSA_MEMORY
    allocateNodeCount_++;
    Node* p = new Node;
    return p;
#else
    Node* address = fixedSizeAllocator_.alloc();

    if (!address)
    {
      return NULL;
    }
    allocateNodeCount_++;
    Node* p = new (address) Node;
    return p;
#endif
  }

  void freeNode(Node* node)
  {

    allocateNodeCount_--;

#if !USE_FSA_MEMORY
    delete node;
#else
    node->~Node();
    fixedSizeAllocator_.free(node);
#endif
  }

private: // data
  // Heap (simple vector but used as a heap, cf. Steve Rabin's game gems article)
  std::vector<Node*> openList_;

  // Closed list is a vector.
  std::vector<Node*> closedList_;

  // Successors is a vector filled out by the user each type successors to a node
  // are generated
  std::vector<Node*> successors_;

  // State
  unsigned int state_;

  // Counts steps
  int stepCount_;

  // Start and goal state pointers
  Node* start_;
  Node* goal_;

  Node* currentSolutionNode_;

#if USE_FSA_MEMORY
  // Memory
  FixedSizeAllocator<Node> fixedSizeAllocator_;
#endif

  // Debug : need to keep these two iterators around
  // for the user Dbg functions
  typename std::vector<Node*>::iterator iterDbgOpen;
  typename std::vector<Node*>::iterator iterDbgClosed;

  // debugging : count memory allocation and free's
  int allocateNodeCount_;

  bool cancelRequest_;
};

#endif
