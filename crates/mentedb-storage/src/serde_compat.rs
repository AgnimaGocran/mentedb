use mentedb_core::MemoryNode;

pub fn serialize_node(node: &MemoryNode) -> (Vec<u8>, Vec<u8>) {
    let mut compact = node.clone();
    let embedding = std::mem::take(&mut compact.embedding);

    let binary = bincode::serialize(&compact)
        .expect("MemoryNode bincode serialization");
    let compressed = lz4_flex::compress_prepend_size(&binary);

    let emb_bytes: Vec<u8> = embedding.iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();

    (compressed, emb_bytes)
}

pub fn deserialize_node(data: &[u8], embedding: &[u8]) -> mentedb_core::MemoryNode {
    let binary = lz4_flex::decompress_size_prepended(data)
        .expect("LZ4 decompression");
    let mut node: MemoryNode = bincode::deserialize(&binary)
        .expect("MemoryNode bincode deserialization");

    node.embedding = embedding.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    node
}
