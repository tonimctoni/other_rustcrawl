# Another crawler in rust

My other crawler written in rust was aweful. While I am still learning, this one is already looking a lot better.


## Usage
Set configuration in `config.json` and run.

## Understanding it
There are no comments, but there is a nice chart showing between which components the data flows, in what direction, and how.
The main function just starts all components.

### Basic inner workings
The core of this program is made up by three components: `Reservoir`, `Client` and `Worker`.

`Reservoir` stores URLs, and streams them to `Client`.

`Client` visits the URLs it gets, and if they lead to either html code or files of interest, they are sent to `Worker`.

`Worker` extracts new URLs from the html code it gets and writes the files of interest to the hard drive.
The extracted URLs are stored in `Reservoir`.

All of these components write data to be reported to `Stats`, which stores it.
`Reporter` is in charge of reporting this data.

Lastly, `Config` loads configuration data from a json file, which is then copied to the relevant components, and `Bloom Filter` keeps track of which URLs have already been sent to `Reservoir` and what files have already been stored on the hard drive.

### Memory consumption
There are three big consumers of memory in this program: two `Bloom Filer`s and a `Reservoir`.
The `Bloom Filer`s consume 512MB of memory each, which does not change, but `Reservoir`'s consumption is unbounded.
This means that, over time, this program should consume increasingly more memory, given that new URLs are found faster than they are looked up.
This is the case when there is more than one new URL per visited site, which is very likely.
However, the proportion of new URLs recognized as new (true positives) decreases as the `BloomFilter` that keeps track of what URLs have already been added to `Reservoir` gets "full" (see the next section).

### `Bloom Filer`'s limit
Which URLs have already been added to `Reservoir` to be visited later is kept track of by a bloom filter.
This bloom filter has a size of 512MB, and uses k=4 hashes.
This means that the false positive rate (new URL's recognized as already added ones) stays below 2% until over 500 million URLs have been added.
After this point though, the false positive rate raises quickly, reaching 50% at 2000 million URLs, and 90% just before 4000 million URLs.
What this means in practice is that this program is bound to stop being effective eventually.
Since this is hard-coded, better hardware would not change when this loss of effectiveness happens.

### How does the shutdown work?
The interrupt signal gets captured by the `Ctrl-C Handler`.
This handler uses its reference to `Reservoir` to close it, dropping the reference (so `Reservoir` can be dropped too eventually).
When closed, `Reservoir`'s streams send `Ok(futures::Async::Ready(None))` when polled, signaling that the stream has ended.
This leads all `Client`s to terminate, dropping their sending ends of the `sync::mpsc::channel`.
When all sending parts of a channel are dropped, an iteration over the receiving part stops.
Thus, the loop within `Worker` ends, leading it to activate a done flag and terminate.
The last reference to `Reservoir` was kept by `Worker`, so it is destroyed.
The done flag causes `Reporter` to terminate after its next report.
This concludes the termination off all processes and destruction of all structures.
Since the main loop of `Reporter` ran on the main thread, its termination means the termination of the program.