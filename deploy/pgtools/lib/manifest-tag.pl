#!/usr/bin/env perl
# Faults: usage, key-missing, key-read, key-nul, key-cr, key-lines, key-line <n>,
# key-size, manifest-read, manifest-size, tag-read, tag-format, tag-size,
# unknown-key, tag-mismatch, output-write. The key-line fault includes its decimal line number.
# Each fault writes one line to stderr.
# CLI: key-ids <key-file>, sign <key-file> <manifest>,
# check <key-file> <manifest> <tag-file>.
# Sign prints the tag line. A successful check prints nothing.
# Key-ids prints one key id per key line, in file order.
use strict;
use warnings;
use Digest::SHA qw(hmac_sha256_hex);
use Errno qw(ENOENT);

# reliability.md §7.11: backup_manifest.max_tag_bytes.
use constant MAX_TAG_BYTES => 4096;
# reliability.md §7.11: backup_manifest.max_file_bytes.
use constant MAX_FILE_BYTES => 65536;
# reliability.md §7.11: backup_manifest.max_keys.
use constant MAX_KEYS => 2;

sub fault {
    my ($name) = @_;
    print STDERR "$name\n";
    exit 1;
}

sub read_file {
    my ($path, $limit, $fault, $size_fault) = @_;
    open my $file, '<:raw', $path
        or fault($fault eq 'key-read' && $! == ENOENT ? 'key-missing' : $fault);
    my $bytes = '';
    while (1) {
        my $count = read $file, my $chunk, $limit + 1 - length($bytes);
        defined $count or fault($fault);
        last if $count == 0;
        $bytes .= $chunk;
        length($bytes) <= $limit or fault($size_fault);
    }
    close $file or fault($fault);
    return $bytes;
}

sub read_keys {
    my ($path) = @_;
    my $bytes = read_file($path, MAX_FILE_BYTES, 'key-read', 'key-size');
    $bytes !~ /\0/ or fault('key-nul');
    $bytes !~ /\r/ or fault('key-cr');
    $bytes =~ s/\n\z//;
    my @lines = split /\n/, $bytes, -1;
    @lines = ('') unless @lines;
    @lines <= MAX_KEYS or fault('key-lines');
    my @keys;
    my $number = 0;
    for my $line (@lines) {
        ++$number;
        $line =~ /\A[0-9a-f]{64}\z/ or fault("key-line $number");
        my $key = pack 'H*', $line;
        push @keys, {
            bytes => $key,
            id => substr(hmac_sha256_hex('recipe-backup-key-id-v1', $key), 0, 16),
        };
    }
    return @keys;
}

sub manifest_tag {
    my ($key, $path) = @_;
    my $manifest = read_file($path, MAX_FILE_BYTES, 'manifest-read', 'manifest-size');
    return hmac_sha256_hex("recipe-backup-manifest-v1\n" . $manifest, $key);
}

# Both arguments hold exactly 64 ASCII hex characters. Visit every byte without an early return.
sub tags_equal {
    my ($expected, $actual) = @_;
    my $different = 0;
    for my $offset (0 .. 63) {
        $different |= ord(substr($expected, $offset, 1)) ^ ord(substr($actual, $offset, 1));
    }
    return $different == 0;
}

sub check_tag {
    my ($keys, $manifest_path, $tag_path) = @_;
    my $line = read_file($tag_path, MAX_TAG_BYTES, 'tag-read', 'tag-size');
    my ($id, $tag) = $line =~ /\Ahmac-sha256-v1 ([0-9a-f]{16}) ([0-9a-f]{64})\n\z/;
    defined $id or fault('tag-format');
    my ($key) = grep { $_->{id} eq $id } @{$keys};
    defined $key or fault('unknown-key');
    tags_equal(manifest_tag($key->{bytes}, $manifest_path), $tag) or fault('tag-mismatch');
}

sub main {
    my ($mode, $key_path, $manifest_path, $tag_path) = @ARGV;
    defined $mode or fault('usage');
    (($mode eq 'sign' && @ARGV == 3) || ($mode eq 'check' && @ARGV == 4)
        || ($mode eq 'key-ids' && @ARGV == 2)) or fault('usage');
    my @keys = read_keys($key_path);
    if ($mode eq 'sign') {
        my $key = $keys[0];
        my $tag = manifest_tag($key->{bytes}, $manifest_path);
        print STDOUT "hmac-sha256-v1 $key->{id} $tag\n" or fault('output-write');
        close(STDOUT) or fault('output-write');
    } elsif ($mode eq 'key-ids') {
        print STDOUT join('', map { "$_->{id}\n" } @keys) or fault('output-write');
        close(STDOUT) or fault('output-write');
    } else {
        check_tag(\@keys, $manifest_path, $tag_path);
    }
}

main();
