# SHA256_Padding_Circuit
Pseudocode Algebraic Circuit design for SHA256 Padding 

## OVERVIEW
The following is the description for an algebraic circuit to perform padding for the SHA256 function, over a variable byte length message of up to 1024 bytes. The INPUTS for the circuit *can* be imagined as simply a buffer array of up to 1024 bytes, along with a variable containing the actual message length in bytes (guaranteed to be less or equal to 1024). We are assuming some convenient syntax (e.g. `=` as equality constraints), and pre-existing implementations of algebraic constraints for SHA256 and other common operations (e.g. `SHA256(block: [u8;64], state: [u8; 32])`, AND operator, SELECT operator, etc...). 



## TESTING
I have come up with a circuit that i think is satisfactory and efficient, by identifying common algebraic patterns for the padding and manually covering all the edge cases with as few conditionals as possible. 
The process of manually identifying the final algebraic constraint is error prone and hard to verify in practice, so I have attached within this repository some simple Rust code that can be used (`cargo run`) to double-check the soundness of the main constraints by simply modifying the value of the message length variable (`len`). Since this design is limited to messages of up to 1024 variables, the constraint could actually be fully verified against the original specification padding algorithm by means of a simple exhaustive search, if one were inclined to write the code for it...

## CIRCUIT LAYOUT
I imagined a Halo2-style execution trace:
- of 18 rows (one for base case recursion values and 17 for hashing of max 1024 bytes plus edge case)
- and 4 column groups
   - one column group for the block to use in hashing, the group contains one column per byte "bj",
   - one column group for the state to use in hashing, i ignore the precise implementation of group columns as that is dependent on SHA256(),
   - one column group for tracking the number of up-to-64 bytes present in the current block that were read from the input array, the group contains just one byte column "b.len",
   - and one column group for the number of bytes currently read in total from the array, the group contains just one field element "bytes_read".
- A selector for all the constraints (except constants) is active on all rows except the first one, I ignored positioning of selectors for constant/fixed cases, but it can easily be just one further selector active on the first row. 

## ASSUMPTIONS
1. i have assumed the ability to query/access bytes of the input array "x" and its length "len" by absolute index value (i.e. `x[i]` and `len[i]`). if these assumptions of absolute indexing are not acceptable one could think, instead of having absolute indexing, a relative indexing where the input array is refitted/copied into columns in parallel to the rest of the circuit, and the same for length columns. this would yield a different circuit design and potentially optimise padding constraints at the cost of 64 more byte columns.
2. i have also changed the length of the input array from a simple usize/field to `[u8; 8]`, since i needed to access individual bytes in the "bj" constraint and since sha256 spec specifies a length of 64-bits. if that is not acceptable, a simple byte decomposition column and constraint can be added to check that the individual bytes can be recomposed into the original input field element.
3. in accordance with the second assumption, i have also implicitly casted from a field element into a byte in the "b.len" constraint. if that is not acceptable, a similar approach can be taken as before, that is to introduce further columns and a simple constraint to track the decomposition of the resulting field element into bytes.
4. i have labelled in the pseudocode the input array as "x", and whenever accessing the current row of a column I have omitted any index (though this can be replaced by `col[row]`), and for the  previous row of a column i have appended "_prev" for simplicity (though this can be replaced by `col[row-1]`). I have further used multiple inequalities in the pseudocode, but ensured that none of them is too large for a lookup table (e.g. "a>b" between bytes) or that they can be efficiently implemented simply by expanding into a reasonably sized constraint (e.g. "a<50" = (a-0)(a-1)...(a-49) for field elements).

## CONSTRAINTS
1. there is a group of 64 constraints for the "bj" column group to ensure that padding is correct and the block is consistent with sha256 spec,
2. there is one constraint for ensuring that the number of input array bytes "b.len" present in the current block is correct,
3. there is one constraint for ensuring that the number of total bytes "bytes_read" currently read from the input array is correct,
4. and there is a final constraint to ensure that the sha256 state "state" remains consistent until the end.
  
There are also a couple of constraints required to fix initial constants in the circuit. Each constraint is defined below in a separate item.

### CONSTRAINT(S) bj: 
This is the constraint for evaluating the block of 64 bytes of data to be fed into an iteration of raw SHA256, which means it has to be padded first.
As reference, padding is defined in sha256 spec as 512-bit multiple: `<msg> 1 <0*> <u64 length>`. The following is a highly optimised formula i obtain by flattening out all the edge cases and grouping common algebraic operations. A description of how I obtained this formula from all the edge cases is lengthy and may follow later.

```Rust
for j in 0..=63:
    bj = SELECT(j < b.len, x[bytes_read_prev + j], A)
    A = SELECT(j == b.len AND b.len_prev == 64, 128, B)
    B = SELECT( 56<= j <=63 AND b.len < 56, len[j-56], 0)
```

#### explanation

In order to find this constraint it's necessary to consider all the possible block patterns we might be in at any current row, as well as their context (what block came before, or will come afterwards). The following is a collection of all the 6 possible patterns:

1. "FULL": in this case the next (or even last) bytes of the message perfectly fit into a block of 64 bytes, which means that we just need to copy over all the bytes (this is the second to last block in the full padding)
   ```Rust
   IF b.len = 64 (AND b.len_prev = 64)
      block = input_block = input_block[0..=63]               
   ```
2. "KINDA FULL": in this case the last bytes of the message are less than a full block (64 bytes), but they are too many to fit the final padding bytes (minimum of 9 bytes) into this block (this is the second to last block in the full padding)
   ```Rust
   IF b.len = 56..=63 (AND b.len_prev = 64)
      block = [input_block[0..=b.len-1], 128, 0*]              
   ```
3. "FINISH KINDA EMPTY": in this case the last few bytes of the message are so few that they can be fitted into the next block along with ALL the final padding bytes (this is the last block in the full padding)
   ```Rust
   IF b.len = 1..=55 (AND b.len_prev = 64)
      block = [input_block[0..=b.len-1], 128, 0*, len[0..=7]]  
   ```
4. then "FINISH FULL": in this case we have 0 bytes left to read from the message, and the previous block was full, so we just have an empty padding block (this is the last block in the full padding)
   ```Rust
   IF b.len = 0 AND b.len_prev = 64
      block = [128, 0{55}, len[0..=7]]                         
   ```
5. then "FINISH KINDA FULL": in this case we have 0 bytes left to read from the message, and the previous block did manage to begin padding but could not complete it, so we just add an empty padding block without the bit marker (this is the last block in the full padding)
   ```Rust
   IF b.len = 0 AND b.len_prev = 56..=63
      block = [0{56}, len[0..=7]]                              
   ```
6. "FINISHED": in this final edge case all the padding has already been completed, we can just ignore what the next block is going to be since don't use it anymore (but the state constraint will need to ensure the state is copied over in this case)
   ```Rust
   IF b.len = 0 AND (b.len_prev = 0 OR b.len_prev = 1..=55)
      block = ?                                                
   ```

The approach I follow here is to identify common algebraic patterns of known fixed size (e.g. `input_block[0..=b.len-1]` or `128`), and try to squash them all with just one same conditional. The other conditionals are positioned strategically to try and push common algebraic patterns into one same conditional (e.g. `128` sometimes is the first byte and sometimes it comes after the input bytes, but in both cases it comes after the input bytes because when it's first there are no input bytes anyway), so that expressions do not repeat each other. Finally, annoying edge cases where we do not know the length of the common algebraic pattern (e.g. `0*`) are kept for last, since the final condition will fail and default to all remaining bytes no matter how many they are. In steps:
1. first consider that when there are bytes to read from the input block we can just read them in
   ```Rust
   bj = SELECT(j < b.len, input_block[j], A)
   ```
   and the condition fails (`A`) either when we are in a situation where we have no input bytes, or when we have pushed the left (`j>=b.len`) index beyond the available input bytes
2. note we have already taken care of the "FULL" case, so we can ignore it and focus on the others. consider that the first remaining index `j==b.len`  is now pointing to the `128` byte in all cases (which, coincidentally, share `b.len_prev=64`) except for the "FINISH KINDA FULL" case. let's take care of this byte:
   ```Rust
   A = SELECT(j==b.len AND b.len_prev==64, 128, B)
   ```
   and the condition fails (`B`) either when the index `j>b.len` is just after the `128` byte in all cases where it exists, or when we are pointing to the next block of `0` bytes in all cases, which is the same thing.
3. now we need to take care of the remaining `0` bytes, but that is complex as we do not know how long they will be, so let us instead take care of the final (`j = (56..=63)`) fixed length `len[0..=7]` bytes, which exist for all cases except whenever we are in the "KINDA FULL" (`b.len = (56..=63)`) case:
   ```Rust
   B = SELECT(56<=j<=63 AND b.len<56, len[j-56], 0)



### CONSTRAINT bytes_read: 
We are ensuring that we are tracking how many bytes we have read in total, up to the limit which is the full length. This is necessary to simplify the constraint for "b.len".

```Rust
bytes_read = SELECT((len - bytes_read_prev) < 64, len, bytes_read_prev + 64)
```

### CONSTRAINT b.len: 
We are ensuring that we know exactly how many bytes we can fit from the input array into the current sha256 padded block. This will never be larger than 64, due to bytes_read constraint and base case constraints.

```Rust
b.len = bytes_read - bytes_read_prev
```

### CONSTRAINT state: 
This is a slightly optimised constraint to ensure that we are always calculating the next iteration of SHA256() when there is a valid block waiting in the current row, otherwise we just copy over the previous state. Of course, in this design document we are assuming `SHA256(block, state)` is a constraint that is already available. Here, "block" is shorthand for `[b0...b63]` columns which we already have. In the unoptimised version, the original condition was `b.len == 0 AND b.len_prev <= 55` but i realised it would hold, as long as i used appropriate base case constraints, even if i used an addition to skip the AND.

```Rust
state = SELECT((b.len + b.len_prev) <= 55, state_prev, SHA256(block, state_prev))
```

### CONSTRAINTS ROW=0 
we are setting the base cases to ensure that the other constraints will remain consistent. 

```Rust
b.len[0] = 64
bytes_read[0] = 0
state[0] = .... (sha256 spec initial state constants)
```

### CONSTRAINTS INPUT/OUTPUT
this ensures that the input length is bounded and the last row of the “state” column contains the sha256 output one would supposedly obtain as circuit input. This is largely irrelevant for the details of this document and would normally be handled outside of this circuit.

```Rust
0 <= len <= 1024
state[17] = output
```

# OPTIMISATIONS: 
i spent a lot of time optimising the constraints, avoding complex operations and flattening out conditionals. still, there are multiple directions for enhancements if one were intent on improving performance.
1. moving the inputs into additional columns parallel to the other columns might assist in copying over data into the correct cells without having to use complex arithmetic to track the padding. it is unclear whether this improves performance because it adds a lot of columns.
2. a more aggressive use of selector columns might assist in further simplifying the padding calculations. As example, the popular implementation of SHA256 in Halo2 by Brechtpd (https://github.com/mabbamOG/zkevm-circuits/blob/sha256/zkevm-circuits/src/sha256_circuit/sha256_bit.rs) shared and optimised across multiple companies, makes heavy use of selectors to identify padding phases, to the point of adding a selector column per word in the block. I am not sure if this is actually more efficient than my version but it's worth a try.
3. compressing the byte operations into (fewer) larger word operations might improve performance, if we can avoid expensive inequality checks that are too large for traditional lookup tables. a rewrite of inequality constraints might help, or switching to newer lookup schemes like Lasso.
4. there are opportunities for further compression of conditionals by exploiting patterns in the constants, like i did for the "state" constraint, but this is something that should be done very carefully and usually requires the use of weird constants in the rest of the circuit (see the 64 init value for the "b.len" column).
 
