"""Rule table for issue 481 calibration. Every rule assumes the candidate set already restricted to
TED Tenders with a version within +-30 days of the DOE version (the census window)."""

def nc(f):          # no hard contradiction: cpv main, deadline (exact), value (exact), internal ref, lot count
    return not f['contra']


def ncs(f):         # no soft contradiction: cpv main, deadline (+-24h), value (+-1%), internal ref, lot count
    return not f['contra_soft']


B = lambda f: f['org']
RULES = [
    # ---- single signals, org-id blocked ----
    ('S0  org', lambda f: B(f)),
    ('S1  org & same_subtype', lambda f: B(f) and f['sub']),
    ('S2  org & dt<=7', lambda f: B(f) and f['dt'] <= 7),
    ('S3  org & cpv_main eq', lambda f: B(f) and f['cpv'] == 'eq'),
    ('S4  org & nuts eq', lambda f: B(f) and f['nuts'] == 'eq'),
    ('S5  org & deadline exact', lambda f: B(f) and f['dl'] == 'eq'),
    ('S6  org & value exact', lambda f: B(f) and f['val'] == 'eq'),
    ('S7  org & internal_ref eq', lambda f: B(f) and f['iref'] == 'eq'),
    ('S8  org & title exact_ci', lambda f: B(f) and f['t_exact']),
    ('S9  org & title norm eq', lambda f: B(f) and f['t_norm']),
] + [
    ('S10 org & title jacc>=%.2f' % t, (lambda t: lambda f: B(f) and f['t_jacc'] >= t)(t))
    for t in (0.9, 0.8, 0.7, 0.6, 0.5)
] + [
    ('S11 org & title ratio>=%.2f' % t, (lambda t: lambda f: B(f) and f['t_ratio'] >= t)(t))
    for t in (0.95, 0.9, 0.85, 0.8)
] + [
    # ---- combinations named in the task ----
    ('C1  org & deadline exact & cpv eq', lambda f: B(f) and f['dl'] == 'eq' and f['cpv'] == 'eq'),
    ('C2  org & cpv eq & deadline exact & value exact', lambda f: B(f) and f['cpv'] == 'eq' and f['dl'] == 'eq' and f['val'] == 'eq'),
    ('C3  org & internal_ref eq & cpv eq', lambda f: B(f) and f['iref'] == 'eq' and f['cpv'] == 'eq'),
    ('C4  org & internal_ref eq & title norm', lambda f: B(f) and f['iref'] == 'eq' and f['t_norm']),
    ('C5  org & title norm & same_subtype', lambda f: B(f) and f['t_norm'] and f['sub']),
    ('C6  org & title norm & same_subtype & dt<=7', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7),
    ('C7  org & title norm & sub & dt<=7 & cpv eq', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and f['cpv'] == 'eq'),
    ('C8  org & title norm & sub & dt<=7 & no-contra', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f)),
    ('C9  org & title norm & sub & dt<=7 & no-contra & cpv eq', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['cpv'] == 'eq'),
    ('C10 org & title norm & sub & dt<=7 & no-contra & iref eq', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['iref'] == 'eq'),
    ('C11 org & title norm & sub & dt<=7 & no-contra & deadline eq', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['dl'] == 'eq'),
    ('C12 org & title norm & sub & dt<=7 & no-contra & (iref|deadline|value eq)', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and 'eq' in (f['iref'], f['dl'], f['val'])),
    ('C13 org & title norm & sub & dt<=7 & no-contra & iref eq & deadline eq', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and f['iref'] == 'eq' and f['dl'] == 'eq'),
    ('C14 org & internal_ref eq & sub & dt<=7 & no-contra', lambda f: B(f) and f['iref'] == 'eq' and f['sub'] and f['dt'] <= 7 and nc(f)),
    ('C15 org & deadline exact & sub & dt<=7 & no-contra', lambda f: B(f) and f['dl'] == 'eq' and f['sub'] and f['dt'] <= 7 and nc(f)),
    ('C16 org & deadline exact & cpv eq & sub & dt<=7 & no-contra', lambda f: B(f) and f['dl'] == 'eq' and f['cpv'] == 'eq' and f['sub'] and f['dt'] <= 7 and nc(f)),
    ('C17 org & title jacc>=0.8 & sub & dt<=7 & no-contra', lambda f: B(f) and f['t_jacc'] >= 0.8 and f['sub'] and f['dt'] <= 7 and nc(f)),
    ('C18 org & title jacc>=0.8 & sub & dt<=7 & no-contra & (iref|deadline eq)', lambda f: B(f) and f['t_jacc'] >= 0.8 and f['sub'] and f['dt'] <= 7 and nc(f) and 'eq' in (f['iref'], f['dl'])),
    ('C19 org & title norm & sub & dt<=7 & no-soft-contra', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and ncs(f)),
    ('C20 org & title norm & sub & dt<=7 & no-soft-contra & (iref|deadline+-1d|value+-1% eq)', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 7 and ncs(f) and 'eq' in (f['iref'], f['dl1d'], f['val1p'])),
    ('C21 org & title norm & sub & dt<=3 & no-contra & (iref|deadline|value eq)', lambda f: B(f) and f['t_norm'] and f['sub'] and f['dt'] <= 3 and nc(f) and 'eq' in (f['iref'], f['dl'], f['val'])),
    ('C22 org & title norm & iref eq & cpv eq & sub & dt<=7 & no-contra', lambda f: B(f) and f['t_norm'] and f['iref'] == 'eq' and f['cpv'] == 'eq' and f['sub'] and f['dt'] <= 7 and nc(f)),
    # name_norm instead of org id (the only blocking available for sdk-0.1)
    ('N1  name & title norm & sub & dt<=7 & no-contra & (iref|deadline|value eq)', lambda f: f['name'] and f['t_norm'] and f['sub'] and f['dt'] <= 7 and nc(f) and 'eq' in (f['iref'], f['dl'], f['val'])),
    ('N2  name & title norm & dt<=7 & no-contra & (iref|deadline eq)  [subtype-free]', lambda f: f['name'] and f['t_norm'] and f['dt'] <= 7 and nc(f) and 'eq' in (f['iref'], f['dl'])),
]
