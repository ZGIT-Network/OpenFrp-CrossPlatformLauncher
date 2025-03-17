export default {
  get(name: string): string | undefined {
    try {
      const value = localStorage.getItem(name);
      if (!value) return undefined;

      try {
        // 尝试解析JSON格式的数据
        const data = JSON.parse(value);
        // 检查是否是带过期时间的数据结构
        if (data && typeof data === 'object' && 'value' in data && 'expires' in data) {
          // 检查是否过期
          if (data.expires && data.expires < Date.now()) {
            // 数据已过期，删除它
            localStorage.removeItem(name);
            return undefined;
          }
          return data.value;
        }
        // 如果不是特定格式，直接返回值
        return value;
      } catch {
        // 如果解析失败，说明是普通字符串，直接返回
        return value;
      }
    } catch (e) {
      console.error('LocalStorage get error:', e);
      return undefined;
    }
  },

  set(name: string, value: string, options: { expires?: number | Date, path?: string } = {}): void {
    try {
      // 如果设置了过期时间，将过期时间和值一起存储
      if (options.expires) {
        const expireDate = options.expires instanceof Date 
          ? options.expires.getTime()
          : Date.now() + options.expires * 864e5;
        
        const data = {
          value,
          expires: expireDate
        };
        localStorage.setItem(name, JSON.stringify(data));
      } else {
        // 如果没有过期时间，直接存储值
        localStorage.setItem(name, value);
      }
    } catch (e) {
      console.error('LocalStorage set error:', e);
    }
  },

  remove(name: string): void {
    try {
      localStorage.removeItem(name);
    } catch (e) {
      console.error('LocalStorage remove error:', e);
    }
  }
};