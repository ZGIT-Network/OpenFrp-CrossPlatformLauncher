import numbro from 'numbro';

export const formatBandwidth = (kbps = 0) => `${Number((kbps / 1000).toFixed(2))} Mbps`;
export const formatTraffic = (mib = 0) =>
  numbro(mib * 1024 * 1024).format({ output: 'byte', base: 'binary', mantissa: 2 });
