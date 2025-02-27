export default {
  get(name: string): string | undefined {
    const value = `; ${document.cookie}`;
    const parts = value.split(`; ${name}=`);
    if (parts.length === 2) {
      return parts.pop()?.split(';').shift();
    }
    return undefined;
  },
  set(name: string, value: string, options: { expires?: number | Date, path?: string } = {}): void {
    let cookieString = `${name}=${value}`;
    
    if (options.expires) {
      const expireDate = options.expires instanceof Date 
        ? options.expires 
        : new Date(Date.now() + options.expires * 864e5);
      cookieString += `; expires=${expireDate.toUTCString()}`;
    }
    
    if (options.path) {
      cookieString += `; path=${options.path}`;
    }
    
    document.cookie = cookieString;
  },
  remove(name: string, options: { path?: string } = {}): void {
    this.set(name, '', { 
      ...options,
      expires: new Date(0) 
    });
  }
};