#!lua name=api_mutex

local function lock(keys, args)
    local key = keys[1]
    local lock_id = args[1]
    local ttl = tonumber(args[2])

    local current = redis.call('get', key)
    if current == lock_id then
        redis.call('expire', key, ttl)
        return 1
    elseif current then
        return 0
    end

    local ok = redis.call('set', key, lock_id, 'NX', 'EX', ttl)
    return ok and 1 or 0
end

local function unlock(keys, args)
    local key = keys[1]
    local lock_id = args[1]

    local current = redis.call('get', key)
    if current == lock_id then
        redis.call('del', key)
        return 1
    end

    return 0
end

redis.register_function('mutex_lock', lock)
redis.register_function('mutex_unlock', unlock)