#ifndef SPSC_RING_HPP
#define SPSC_RING_HPP

#include <atomic>
#include <cstddef>
#include <stdexcept>

/**
 * Wait- and lock-free, single-producer single-consumer, fixed-size ring buffer.
 * 
 * @see http://www.codeproject.com/Articles/43510/Lock-Free-Single-Producer-Single-Consumer-Circular
 */
template< typename T, std::size_t Size >
class SpscRing
{
public :
  /**
   * Default constructor.
   * 
   * constructs an empty ring buffer.
   */
  SpscRing(): head_( 0u ), tail_( 0u )
  {
    // Do nothing.
  }
  /**
   * Copy constructor.
   *
   * TODO: WARNING: does not copy !!!
   */
  SpscRing(SpscRing const&): head_( 0u ), tail_( 0u )
  {
    // Do nothing but warn the user.
    throw std::runtime_error("Trying to use unimplemented copy constructor of SpscRing!");
  }

  /**
   * Inserts data element into the ring if the ring is empty.
   * 
   * @param t Data to insert.
   * 
   * @return On success @c true is returned and if the ring was full @c false is returned.
   */
  bool push( T const& t )
  {
    auto const curTail  = tail_.load( std::memory_order_relaxed );
    auto const nextTail = increment( curTail );
    
    if ( nextTail != head_.load( std::memory_order_acquire ) )
    {
      data_[curTail] = t;
      tail_.store( nextTail, std::memory_order_release );
      
      return true;
    }
    
    return false; // Buffer is full.
  }
  
  /**
   * Retrieves data from the ring if it is not empty.
   * 
   * @param[out] t Data retrieved is stored in this parameter.
   * 
   * @return On success @c true is returned while @c false is returned is the queue was empty.
   */
  bool pop( T& t )
  {
    auto const curHead = head_.load( std::memory_order_relaxed );
    
    if ( curHead != tail_.load( std::memory_order_acquire ) )
    {
      t = data_[curHead];
      head_.store( increment( curHead ), std::memory_order_release );
      
      return true;
    }
    
    return false; // Buffer is empty.
  }
  
private :
  enum { 
    Capacity = Size + 1u ///< Capacity of the ring buffer.
  };
  
  /**
   * Increment with wrap-around.
   * 
   * @param index Value to increment by one.
   * 
   * @return @code (index + 1) % Capacity @endcode
   */
  static std::size_t increment( std::size_t index )
  { 
    return ++index % Capacity; 
  }
  
  std::atomic< std::size_t > head_; ///< Index of the head.
  std::atomic< std::size_t > tail_; ///< Index of the tail.
  T                          data_[Capacity]; ///< Buffer storage.
};

#endif
