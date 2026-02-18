// AOEE Java Skeleton (illustrative)
import java.util.*;
import java.util.concurrent.*;
import java.util.concurrent.locks.*;

record EdgeKey(long src, int type) {}

final class Segment {
  final byte[] bytes;
  final int first, last, count;
  Segment(byte[] bytes, int first, int last, int count) {
    this.bytes = bytes; this.first = first; this.last = last; this.count = count;
  }
}

final class PostingList {
  final ArrayList<Integer> buffer = new ArrayList<>(64);
  final ArrayList<Integer> tombstones = new ArrayList<>(16);
  volatile List<Segment> segments = List.of(); // immutable snapshot
  final ReentrantReadWriteLock lock = new ReentrantReadWriteLock();
}

final class AoeeShard {
  private final ConcurrentHashMap<EdgeKey, PostingList> map = new ConcurrentHashMap<>();

  public void addEdge(long src, int type, int dst) {
    PostingList pl = map.computeIfAbsent(new EdgeKey(src,type), k -> new PostingList());
    pl.lock.writeLock().lock();
    try {
      pl.buffer.add(dst);
      // if buffer too big => schedule compaction
    } finally {
      pl.lock.writeLock().unlock();
    }
  }

  static int[] intersectSorted(int[] a, int[] b) {
    int i=0,j=0; int[] tmp = new int[Math.min(a.length,b.length)]; int k=0;
    while (i<a.length && j<b.length) {
      int av=a[i], bv=b[j];
      if (av==bv) { tmp[k++]=av; i++; j++; }
      else if (av<bv) i++;
      else j++;
    }
    return Arrays.copyOf(tmp, k);
  }
}
